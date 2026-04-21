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
use elemental_schema::quilt::ProfileJson;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::{
        quilt::{
            config::QuiltLaunchConfig,
            prepared::{
                LaunchedQuiltVersion, PreparedQuiltVersion, QuiltRemoteResolver,
                ResolvedQuiltMetadata, ResolvedQuiltVersion,
            },
            source::QuiltSource,
        },
        vanilla::source::VanillaSource,
    },
    families::version_json::{
        PASSTHROUGH_PROFILE_BEHAVIOR, VersionJsonInstanceLayout, VersionJsonRootLayout,
        builder::VersionJsonLaunchBuilder, merge_profile_with_behavior,
    },
    inspect::InstanceProbe,
    runtime::resolve_runtime,
};

const QUILT_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "quilt",
    name: "Quilt",
};

pub struct QuiltDriver {
    source: QuiltSource,
    vanilla_source: VanillaSource,
    downloader: Arc<ElementalDownloader>,
}

impl QuiltDriver {
    pub fn new(
        source: QuiltSource,
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
            QuiltSource::default(),
            VanillaSource::default(),
            ElementalDownloader::with_config_default()
                .context("create default elemental downloader failed")?,
        ))
    }

    pub fn source(&self) -> &QuiltSource {
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
    ) -> Result<PreparedQuiltVersion<L, VL>> {
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
    ) -> Result<PreparedQuiltVersion<L, VL>> {
        ResolvedQuiltVersion::load(self.remote_resolver(), instance.clone())?
            .into_prepared()
            .await
    }

    pub async fn launch<
        A,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        prepared_version: PreparedQuiltVersion<L, VL>,
        config: &QuiltLaunchConfig,
        authorizer: A,
    ) -> Result<LaunchedQuiltVersion<L, VL>>
    where
        A: Authorizer,
    {
        let (runtime, command) = self
            .build_launch_command(authorizer, &prepared_version, config)
            .await?;
        let child = process::spawn_command(command)?;

        Ok(LaunchedQuiltVersion {
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
        prepared_version: &PreparedQuiltVersion<L, VL>,
        config: &QuiltLaunchConfig,
    ) -> Result<(Distribution, LaunchCommand)>
    where
        A: Authorizer,
    {
        let runtime = self
            .runtime_for_prepared_version(
                prepared_version,
                config.runtime_major_version,
                config.runtime_executable_path.as_deref(),
            )
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
        prepared_version: &PreparedQuiltVersion<L, VL>,
        runtime_major_version: Option<usize>,
        runtime_executable_path: Option<&std::path::Path>,
    ) -> Result<Distribution> {
        let required_major_version =
            runtime_major_version.unwrap_or_else(|| prepared_version.required_java_major_version());

        resolve_runtime(required_major_version, runtime_executable_path, "launch").await
    }

    fn build_launch_builder<
        A,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        authorizer: A,
        runtime: Distribution,
        prepared_version: &PreparedQuiltVersion<L, VL>,
        config: &QuiltLaunchConfig,
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

    fn remote_resolver(&self) -> QuiltRemoteResolver {
        QuiltRemoteResolver::new(
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
    ) -> Result<ResolvedQuiltVersion<L, VL>> {
        if let Ok(resolved) = ResolvedQuiltVersion::load(self.remote_resolver(), instance.clone()) {
            let status = resolved.status().await?;
            if status.is_downloaded()
                && !local_metadata_needs_refresh(
                    &resolved.metadata,
                    game_version.as_str(),
                    loader_version.as_str(),
                )
            {
                return Ok(resolved);
            }
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
    ) -> Result<ResolvedQuiltVersion<L, VL>> {
        self.resolve_metadata(game_version, loader_version)
            .await?
            .persist(instance)
            .await
    }

    async fn resolve_metadata(
        &self,
        game_version: String,
        loader_version: String,
    ) -> Result<ResolvedQuiltMetadata> {
        let base_metadata = self.resolve_vanilla_metadata(game_version.clone()).await?;
        let profile = self
            .source
            .profile_json(game_version.as_str(), loader_version.as_str())
            .await?;
        let metadata = merge_profile(profile, base_metadata.metadata)?;

        Ok(ResolvedQuiltMetadata::new(
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
impl<L: Layout, VL: Layout> Driver<L, VL> for QuiltDriver {
    fn descriptor(&self) -> DriverDescriptor {
        QUILT_DRIVER
    }

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };
        let Some(driver_version) = inspect_driver_version(metadata) else {
            return Ok(None);
        };

        Ok(Some(InstalledDriver {
            driver: QUILT_DRIVER,
            driver_version: Some(driver_version),
            game_version: metadata
                .inherits_from
                .clone()
                .or_else(|| Some(metadata.id.clone())),
            description: Some(metadata.release_type.clone()),
        }))
    }
}

fn merge_profile(
    profile: ProfileJson,
    base_metadata: crate::families::version_json::PistonMetaData,
) -> Result<crate::families::version_json::PistonMetaData> {
    merge_profile_with_behavior(&PASSTHROUGH_PROFILE_BEHAVIOR, base_metadata, profile)
}

fn local_metadata_needs_refresh(
    metadata: &crate::families::version_json::PistonMetaData,
    game_version: &str,
    loader_version: &str,
) -> bool {
    let expected_id = format!("quilt-loader-{loader_version}-{game_version}");
    metadata.id != expected_id
        || metadata.inherits_from.as_deref() != Some(game_version)
        || inspect_driver_version(metadata).is_none_or(|installed| installed != loader_version)
}

fn inspect_driver_version(
    metadata: &crate::families::version_json::PistonMetaData,
) -> Option<String> {
    metadata
        .libraries
        .iter()
        .map(|library| library.name.as_str())
        .find(|name| name.starts_with("org.quiltmc:quilt-loader:"))
        .and_then(|name| name.split(':').nth(2).map(ToOwned::to_owned))
}
