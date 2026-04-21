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
use elemental_schema::{mojang::piston::PistonMetaData, quilt::ProfileJson};

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
        vanilla::source::{VanillaSource, resolve_vanilla_metadata},
    },
    families::version_json::{
        PASSTHROUGH_PROFILE_BEHAVIOR, VersionJsonInstanceLayout, VersionJsonRootLayout,
        build_version_json_launch_command, launch_version_json_instance,
        load_prepared_version_json, merge_profile_with_behavior, persist_version_json,
        prepare_version_json,
    },
    inspect::{InstanceProbe, find_library_version, inspect_driver_version_from_libraries},
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
        prepare_version_json(self.downloader(), || {
            self.resolve_or_load(instance, game_version, loader_version)
        })
        .await
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
        persist_version_json(instance, || {
            self.resolve_metadata(game_version, loader_version)
        })
        .await
    }

    async fn resolve_metadata(
        &self,
        game_version: String,
        loader_version: String,
    ) -> Result<ResolvedQuiltMetadata> {
        let base_metadata =
            resolve_vanilla_metadata(self.vanilla_source(), game_version.as_str()).await?;
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
        Ok(inspect_driver_version_from_libraries(
            metadata,
            QUILT_DRIVER,
            &["org.quiltmc:quilt-loader:"],
        ))
    }
}

fn merge_profile(profile: ProfileJson, base_metadata: PistonMetaData) -> Result<PistonMetaData> {
    merge_profile_with_behavior(&PASSTHROUGH_PROFILE_BEHAVIOR, base_metadata, profile)
}

fn local_metadata_needs_refresh(
    metadata: &PistonMetaData,
    game_version: &str,
    loader_version: &str,
) -> bool {
    let expected_id = format!("quilt-loader-{loader_version}-{game_version}");
    metadata.id != expected_id
        || metadata.inherits_from.as_deref() != Some(game_version)
        || inspect_driver_version(metadata).is_none_or(|installed| installed != loader_version)
}

fn inspect_driver_version(metadata: &PistonMetaData) -> Option<String> {
    find_library_version(metadata, &["org.quiltmc:quilt-loader:"])
}
