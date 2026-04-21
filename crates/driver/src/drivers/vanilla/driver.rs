use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use elemental_core::{
    auth::authorizer::Authorizer,
    launcher::{command::LaunchCommand, process},
    runtime::distribution::Distribution,
    storage::{Storage, layout::Layout},
};
use elemental_infra::downloader::core::ElementalDownloader;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::{
        vanilla::{
            config::VanillaLaunchConfig,
            prepared::{
                LaunchedVanillaVersion, PreparedVanillaVersion, ResolvedVanillaMetadata,
                ResolvedVanillaVersion,
            },
            source::VanillaSource,
        },
        version_json::{
            PistonMetaData, VersionJsonInstanceLayout, VersionJsonRootLayout,
            builder::VersionJsonLaunchBuilder,
        },
    },
    inspect::InstanceProbe,
};

pub struct VanillaDriver {
    source: VanillaSource,
    downloader: Arc<ElementalDownloader>,
}

impl VanillaDriver {
    pub fn new(source: VanillaSource, downloader: Arc<ElementalDownloader>) -> Self {
        Self { source, downloader }
    }

    pub fn with_defaults() -> Result<Self> {
        Ok(Self::new(
            VanillaSource::default(),
            ElementalDownloader::with_config_default()
                .context("create default elemental downloader failed")?,
        ))
    }

    pub fn source(&self) -> &VanillaSource {
        &self.source
    }

    pub fn downloader(&self) -> &ElementalDownloader {
        self.downloader.as_ref()
    }

    pub async fn prepare<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
        version_id: String,
    ) -> Result<PreparedVanillaVersion<L, VL>> {
        let resolved = self.resolve_or_load(instance, version_id).await?;
        resolved.prepare(self.downloader()).await
    }

    pub async fn load_prepared<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedVanillaVersion<L, VL>> {
        ResolvedVanillaVersion::load(self.source.endpoints().clone(), instance.clone())?
            .into_prepared()
            .await
    }

    pub async fn launch<
        A,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        prepared_version: PreparedVanillaVersion<L, VL>,
        config: &VanillaLaunchConfig,
        authorizer: A,
    ) -> Result<LaunchedVanillaVersion<L, VL>>
    where
        A: Authorizer,
    {
        let (runtime, command) = self
            .build_launch_command(authorizer, &prepared_version, config)
            .await?;
        let child = process::spawn_command(command)?;

        Ok(LaunchedVanillaVersion {
            prepared_version,
            runtime,
            child,
        })
    }

    pub async fn build_launch_command<
        A,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        authorizer: A,
        prepared_version: &PreparedVanillaVersion<L, VL>,
        config: &VanillaLaunchConfig,
    ) -> Result<(Distribution, LaunchCommand)>
    where
        A: Authorizer,
    {
        let runtime = self
            .runtime_for_prepared_version(prepared_version, config.runtime_major_version)
            .await?;
        let command = self
            .build_launch_builder(authorizer, runtime.clone(), prepared_version, config)?
            .build_command()
            .await?;

        Ok((runtime, command))
    }

    async fn runtime_for_prepared_version<
        L: VersionJsonRootLayout,
        VL: VersionJsonInstanceLayout,
    >(
        &self,
        prepared_version: &PreparedVanillaVersion<L, VL>,
        runtime_major_version: Option<usize>,
    ) -> Result<Distribution> {
        let required_major_version =
            runtime_major_version.unwrap_or_else(|| prepared_version.required_java_major_version());

        Distribution::find_cached_by_java_major(required_major_version)
            .await
            .with_context(|| {
                format!(
                    "can't find a local Java runtime with major version {}",
                    required_major_version
                )
            })
    }

    fn build_launch_builder<
        A,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        authorizer: A,
        runtime: Distribution,
        prepared_version: &PreparedVanillaVersion<L, VL>,
        config: &VanillaLaunchConfig,
    ) -> Result<VersionJsonLaunchBuilder<A, L, VL>>
    where
        A: Authorizer,
    {
        let mut builder = VersionJsonLaunchBuilder::new(
            authorizer,
            runtime,
            prepared_version.resolved_version.version.clone(),
        );

        if let Some(client_id) = &config.client_id {
            builder = builder.set_client_id(client_id.clone());
        }

        if let Some(resolution) = &config.resolution {
            builder = builder.set_resolution(resolution.width.clone(), resolution.height.clone());
        }

        if let (Some(name), Some(version)) = (&config.launcher_name, &config.launcher_version) {
            builder = builder.set_launcher(name.clone(), version.clone());
        }

        if let Some(quick_play) = &config.quick_play {
            builder = builder.set_quick_play(
                quick_play.path.clone(),
                quick_play.multiplayer.clone(),
                quick_play.singleplayer.clone(),
                quick_play.realms.clone(),
            );
        }

        if !config.extra_jvm_arguments.is_empty() {
            builder = builder.set_extra_jvm_arguments(config.extra_jvm_arguments.clone());
        }

        if !config.extra_game_arguments.is_empty() {
            builder = builder.set_extra_game_arguments(config.extra_game_arguments.clone());
        }

        Ok(builder)
    }

    async fn resolve_or_load<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
        version_id: String,
    ) -> Result<ResolvedVanillaVersion<L, VL>> {
        if let Ok(resolved) =
            ResolvedVanillaVersion::load(self.source.endpoints().clone(), instance.clone())
        {
            return Ok(resolved);
        }

        self.resolve_version(instance, version_id).await
    }

    async fn resolve_version<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
        version_id: String,
    ) -> Result<ResolvedVanillaVersion<L, VL>> {
        self.resolve_metadata(version_id)
            .await?
            .persist(instance)
            .await
    }

    async fn resolve_metadata(&self, version_id: String) -> Result<ResolvedVanillaMetadata> {
        let launchmeta = self.source.launch_meta().await?;
        let metadata_url = launchmeta
            .versions
            .iter()
            .find(|version| version.id == version_id)
            .context(format!("Can't find version named `{version_id}`"))?
            .url
            .clone();
        let metadata = self.source.piston_meta(metadata_url).await?;
        let asset_index_objects = self
            .source
            .asset_index_objects(&metadata.asset_index.url)
            .await?;

        Ok(ResolvedVanillaMetadata::new(
            self.source.endpoints().clone(),
            metadata,
            asset_index_objects,
        ))
    }
}

#[async_trait]
impl<L: Layout, VL: Layout> Driver<L, VL> for VanillaDriver {
    fn descriptor(&self) -> DriverDescriptor {
        DriverDescriptor {
            id: "vanilla",
            name: "Vanilla",
        }
    }

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };
        if has_loader_marker(metadata) {
            return Ok(None);
        }

        Ok(Some(InstalledDriver {
            driver: <Self as Driver<L, VL>>::descriptor(self),
            driver_version: None,
            game_version: Some(metadata.id.clone()),
            description: Some(metadata.release_type.clone()),
        }))
    }
}

fn has_loader_marker(metadata: &PistonMetaData) -> bool {
    metadata.libraries.iter().any(|library| {
        let name = library.name.as_str();
        name.starts_with("net.minecraftforge:forge:")
            || name.starts_with("net.neoforged:forge:")
            || name.starts_with("net.neoforged:neoforge:")
            || name.starts_with("net.fabricmc:fabric-loader:")
    })
}
