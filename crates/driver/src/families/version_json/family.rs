use std::{fmt::Debug, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_infra::downloader::core::ElementalDownloader;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    driver::{DriverDescriptor, InstalledDriver},
    drivers::vanilla::source::VanillaSource,
    loader_version::LoaderVersionId,
};

use super::{VersionJsonRemoteResolver, driver::ProfiledVersionJsonDriver};

#[async_trait(?Send)]
pub trait ProfiledVersionJsonFamily: Clone + Debug + Send + Sync + 'static {
    type Source: Clone + Debug + Send + Sync + 'static;
    type Profile: Send;
    type RemoteResolver: VersionJsonRemoteResolver + Clone + Send + Sync + 'static;

    fn descriptor(&self) -> DriverDescriptor;

    fn default_source(&self) -> Result<Self::Source>;

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver;

    async fn profile(
        &self,
        source: &Self::Source,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<Self::Profile>;

    fn merge_profile(
        &self,
        base_metadata: PistonMetaData,
        profile: Self::Profile,
    ) -> Result<PistonMetaData>;

    fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool;

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver>;
}

pub trait ProfiledVersionJsonFamilyExt: ProfiledVersionJsonFamily + Sized {
    fn build_driver(
        self,
        source: Self::Source,
        vanilla_source: VanillaSource,
        downloader: Arc<ElementalDownloader>,
    ) -> ProfiledVersionJsonDriver<Self> {
        ProfiledVersionJsonDriver::new(self, source, vanilla_source, downloader)
    }

    fn build_driver_with_defaults(self) -> Result<ProfiledVersionJsonDriver<Self>> {
        ProfiledVersionJsonDriver::with_defaults(self)
    }
}

impl<F> ProfiledVersionJsonFamilyExt for F where F: ProfiledVersionJsonFamily {}
