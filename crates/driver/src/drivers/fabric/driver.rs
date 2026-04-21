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

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::{
        fabric::{
            config::FabricLaunchConfig,
            flavors::flavor_spec,
            prepared::{
                FabricRemoteResolver, LaunchedFabricVersion, PreparedFabricVersion,
                ResolvedFabricMetadata, ResolvedFabricVersion,
            },
            source::{FabricEndpointOverrides, FabricFlavor, FabricSource},
        },
        shared::{
            build_version_json_launch_command, installed_version_json_driver,
            launch_version_json_instance, load_prepared_version_json, resolve_vanilla_metadata,
        },
        vanilla::source::VanillaSource,
    },
    families::version_json::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    inspect::InstanceProbe,
};

pub struct FabricDriver {
    flavor: FabricFlavor,
    source: FabricSource,
    vanilla_source: VanillaSource,
    downloader: Arc<ElementalDownloader>,
}

impl FabricDriver {
    pub fn new(
        flavor: FabricFlavor,
        source: FabricSource,
        vanilla_source: VanillaSource,
        downloader: Arc<ElementalDownloader>,
    ) -> Self {
        Self {
            flavor,
            source,
            vanilla_source,
            downloader,
        }
    }

    pub fn with_defaults() -> Result<Self> {
        Self::for_flavor(FabricFlavor::Fabric)
    }

    pub fn for_flavor(flavor: FabricFlavor) -> Result<Self> {
        Ok(Self::new(
            flavor.clone(),
            FabricSource::for_flavor(flavor)?,
            VanillaSource::default(),
            ElementalDownloader::with_config_default()
                .context("create default elemental downloader failed")?,
        ))
    }

    pub fn legacy_fabric() -> Result<Self> {
        Self::for_flavor(FabricFlavor::LegacyFabric)
    }

    pub fn babric() -> Result<Self> {
        Self::for_flavor(FabricFlavor::Babric)
    }

    pub fn with_overrides(
        flavor: FabricFlavor,
        overrides: FabricEndpointOverrides,
    ) -> Result<Self> {
        Ok(Self::new(
            flavor,
            FabricSource::with_overrides(overrides)?,
            VanillaSource::default(),
            ElementalDownloader::with_config_default()
                .context("create default elemental downloader failed")?,
        ))
    }

    pub fn source(&self) -> &FabricSource {
        &self.source
    }

    pub fn flavor(&self) -> &FabricFlavor {
        &self.flavor
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
        load_prepared_version_json(self.remote_resolver(), instance).await
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
        launch_version_json_instance(authorizer, prepared_version, config).await
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
        build_version_json_launch_command(authorizer, prepared_version, config).await
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
            let status = resolved.status().await?;
            let flavor = flavor_spec(self.flavor());
            if status.is_downloaded()
                && !flavor.local_metadata_needs_refresh(
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
        let metadata = flavor_spec(self.flavor()).merge_profile(base_metadata.metadata, profile)?;

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
        resolve_vanilla_metadata(self.vanilla_source(), game_version.as_str()).await
    }
}

#[async_trait]
impl<L: Layout, VL: Layout> Driver<L, VL> for FabricDriver {
    fn descriptor(&self) -> DriverDescriptor {
        flavor_spec(self.flavor()).descriptor()
    }

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };
        let flavor = flavor_spec(self.flavor());
        let Some(driver_version) = flavor.inspect_driver_version(metadata) else {
            return Ok(None);
        };

        Ok(Some(installed_version_json_driver(
            metadata,
            flavor.descriptor(),
            Some(driver_version),
        )))
    }
}
