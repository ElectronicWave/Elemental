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
        fabric::{
            config::FabricLaunchConfig,
            prepared::{
                FabricRemoteResolver, LaunchedFabricVersion, PreparedFabricVersion,
                ResolvedFabricMetadata, ResolvedFabricVersion, merge_fabric_profile,
            },
            source::FabricSource,
        },
        vanilla::source::VanillaSource,
        version_json::{
            VersionJsonInstanceLayout, VersionJsonRootLayout, builder::VersionJsonLaunchBuilder,
        },
    },
    inspect::InstanceProbe,
};

pub struct FabricDriver {
    source: FabricSource,
    vanilla_source: VanillaSource,
    downloader: Arc<ElementalDownloader>,
}

impl FabricDriver {
    pub fn new(
        source: FabricSource,
        vanilla_source: VanillaSource,
        downloader: Arc<ElementalDownloader>,
    ) -> Self {
        Self {
            source,
            vanilla_source,
            downloader,
        }
    }

    pub fn with_defaults() -> Result<Self> {
        Ok(Self::new(
            FabricSource::default(),
            VanillaSource::default(),
            ElementalDownloader::with_config_default()
                .context("create default elemental downloader failed")?,
        ))
    }

    pub fn source(&self) -> &FabricSource {
        &self.source
    }

    pub fn vanilla_source(&self) -> &VanillaSource {
        &self.vanilla_source
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
        game_version: String,
        loader_version: String,
    ) -> Result<PreparedFabricVersion<L, VL>> {
        let resolved = self
            .resolve_or_load(instance, game_version, loader_version)
            .await?;
        resolved.prepare(self.downloader()).await
    }

    pub async fn load_prepared<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedFabricVersion<L, VL>> {
        ResolvedFabricVersion::load(self.remote_resolver(), instance.clone())?
            .into_prepared()
            .await
    }

    pub async fn launch<
        A,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        prepared_version: PreparedFabricVersion<L, VL>,
        config: &FabricLaunchConfig,
        authorizer: A,
    ) -> Result<LaunchedFabricVersion<L, VL>>
    where
        A: Authorizer,
    {
        let (runtime, command) = self
            .build_launch_command(authorizer, &prepared_version, config)
            .await?;
        let child = process::spawn_command(command)?;

        Ok(LaunchedFabricVersion {
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
        prepared_version: &PreparedFabricVersion<L, VL>,
        config: &FabricLaunchConfig,
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
        prepared_version: &PreparedFabricVersion<L, VL>,
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
        prepared_version: &PreparedFabricVersion<L, VL>,
        config: &FabricLaunchConfig,
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

        Ok(builder)
    }

    fn remote_resolver(&self) -> FabricRemoteResolver {
        FabricRemoteResolver::new(
            self.vanilla_source.endpoints().clone(),
            self.source.endpoints().clone(),
        )
    }

    async fn resolve_or_load<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
        game_version: String,
        loader_version: String,
    ) -> Result<ResolvedFabricVersion<L, VL>> {
        if let Ok(resolved) = ResolvedFabricVersion::load(self.remote_resolver(), instance.clone())
        {
            return Ok(resolved);
        }

        self.resolve_version(instance, game_version, loader_version)
            .await
    }

    async fn resolve_version<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
        game_version: String,
        loader_version: String,
    ) -> Result<ResolvedFabricVersion<L, VL>> {
        self.resolve_metadata(game_version, loader_version)
            .await?
            .persist(instance)
            .await
    }

    async fn resolve_metadata(
        &self,
        game_version: String,
        loader_version: String,
    ) -> Result<ResolvedFabricMetadata> {
        let base_metadata = self.resolve_vanilla_metadata(game_version.clone()).await?;
        let profile = self
            .source
            .profile_json(game_version.as_str(), loader_version.as_str())
            .await?;
        let metadata = merge_fabric_profile(base_metadata.metadata, profile)?;

        Ok(ResolvedFabricMetadata::new(
            self.remote_resolver(),
            metadata,
            base_metadata.asset_index_objects,
        ))
    }

    async fn resolve_vanilla_metadata(
        &self,
        game_version: String,
    ) -> Result<crate::drivers::vanilla::prepared::ResolvedVanillaMetadata> {
        let launchmeta = self.vanilla_source.launch_meta().await?;
        let metadata_url = launchmeta
            .versions
            .iter()
            .find(|version| version.id == game_version)
            .context(format!("Can't find version named `{game_version}`"))?
            .url
            .clone();
        let metadata = self.vanilla_source.piston_meta(metadata_url).await?;
        let asset_index_objects = self
            .vanilla_source
            .asset_index_objects(&metadata.asset_index.url)
            .await?;

        Ok(
            crate::drivers::vanilla::prepared::ResolvedVanillaMetadata::new(
                self.vanilla_source.endpoints().clone(),
                metadata,
                asset_index_objects,
            ),
        )
    }
}

#[async_trait]
impl<L: Layout, VL: Layout> Driver<L, VL> for FabricDriver {
    fn descriptor(&self) -> DriverDescriptor {
        DriverDescriptor {
            id: "fabric",
            name: "Fabric",
        }
    }

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };
        let library_name = metadata
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .find(|name| name.starts_with("net.fabricmc:fabric-loader:"));

        let Some(library_name) = library_name else {
            return Ok(None);
        };

        let driver_version = library_name.split(':').nth(2).map(ToOwned::to_owned);

        Ok(Some(InstalledDriver {
            driver: <Self as Driver<L, VL>>::descriptor(self),
            driver_version,
            game_version: metadata
                .inherits_from
                .clone()
                .or_else(|| Some(metadata.id.clone())),
            description: Some(metadata.release_type.clone()),
        }))
    }
}
