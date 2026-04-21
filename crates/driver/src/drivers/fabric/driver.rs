use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use elemental_infra::downloader::core::ElementalDownloader;
use elemental_schema::{fabric::ProfileJson, mojang::piston::PistonMetaData};

use crate::{
    driver::{DriverDescriptor, InstalledDriver},
    drivers::{
        fabric::{
            flavors::flavor_spec,
            source::{FabricEndpointOverrides, FabricFlavor, FabricSource},
        },
        vanilla::source::VanillaSource,
    },
    families::version_json::{ProfiledVersionJsonDriver, ProfiledVersionJsonFamily},
    inspect::installed_version_json_driver,
};

#[derive(Debug, Clone)]
pub struct FabricDriverFamily {
    flavor: FabricFlavor,
}

impl FabricDriverFamily {
    pub fn new(flavor: FabricFlavor) -> Self {
        Self { flavor }
    }

    fn flavor(&self) -> &FabricFlavor {
        &self.flavor
    }

    pub fn new_driver(
        &self,
        source: FabricSource,
        vanilla_source: VanillaSource,
        downloader: Arc<ElementalDownloader>,
    ) -> FabricDriver {
        ProfiledVersionJsonDriver::new(self.clone(), source, vanilla_source, downloader)
    }

    pub fn new_driver_with_defaults(&self) -> Result<FabricDriver> {
        ProfiledVersionJsonDriver::with_defaults(self.clone())
    }

    pub fn new_driver_with_overrides(
        &self,
        overrides: FabricEndpointOverrides,
    ) -> Result<FabricDriver> {
        Ok(self.new_driver(
            FabricSource::with_overrides(overrides)?,
            VanillaSource::default(),
            ElementalDownloader::with_config_default()
                .context("create default elemental downloader failed")?,
        ))
    }
}

#[async_trait(?Send)]
impl ProfiledVersionJsonFamily for FabricDriverFamily {
    type Source = FabricSource;
    type Profile = ProfileJson;
    type RemoteResolver = super::prepared::FabricRemoteResolver;

    fn descriptor(&self) -> DriverDescriptor {
        flavor_spec(self.flavor()).descriptor()
    }

    fn default_source(&self) -> Result<Self::Source> {
        FabricSource::for_flavor(self.flavor.clone())
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver {
        super::prepared::FabricRemoteResolver::new(
            vanilla_source.endpoints().clone(),
            source.endpoints().clone(),
        )
    }

    async fn profile(
        &self,
        source: &Self::Source,
        game_version: &str,
        loader_version: &str,
    ) -> Result<Self::Profile> {
        source.profile_json(game_version, loader_version).await
    }

    fn merge_profile(
        &self,
        base_metadata: PistonMetaData,
        profile: Self::Profile,
    ) -> Result<PistonMetaData> {
        flavor_spec(self.flavor()).merge_profile(base_metadata, profile)
    }

    fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &str,
        loader_version: &str,
    ) -> bool {
        flavor_spec(self.flavor()).local_metadata_needs_refresh(
            metadata,
            game_version,
            loader_version,
        )
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        let flavor = flavor_spec(self.flavor());
        let driver_version = flavor.inspect_driver_version(metadata)?;

        Some(installed_version_json_driver(
            metadata,
            flavor.descriptor(),
            Some(driver_version),
        ))
    }
}

pub type FabricDriver = ProfiledVersionJsonDriver<FabricDriverFamily>;
