use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use elemental_core::{
    auth::authorizer::Authorizer,
    launcher::command::LaunchCommand,
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
        shared::{
            build_version_json_launch_command, find_library_version, installed_version_json_driver,
            launch_version_json_instance, load_prepared_version_json, resolve_vanilla_metadata,
        },
        vanilla::source::VanillaSource,
    },
    families::version_json::{
        PASSTHROUGH_PROFILE_BEHAVIOR, VersionJsonInstanceLayout, VersionJsonRootLayout,
        merge_profile_with_behavior,
    },
    inspect::InstanceProbe,
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
        load_prepared_version_json(self.remote_resolver(), instance).await
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
        launch_version_json_instance(authorizer, prepared_version, config).await
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
        build_version_json_launch_command(authorizer, prepared_version, config).await
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
        resolve_vanilla_metadata(self.vanilla_source(), game_version.as_str()).await
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

        Ok(Some(installed_version_json_driver(
            metadata,
            QUILT_DRIVER,
            Some(driver_version),
        )))
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
    find_library_version(metadata, &["org.quiltmc:quilt-loader:"])
}
