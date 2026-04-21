use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use elemental_infra::downloader::core::ElementalDownloader;
use elemental_schema::{mojang::piston::PistonMetaData, quilt::ProfileJson};

use crate::{
    driver::{DriverDescriptor, InstalledDriver},
    drivers::{quilt::source::QuiltSource, vanilla::source::VanillaSource},
    families::version_json::{
        PASSTHROUGH_PROFILE_BEHAVIOR, ProfiledVersionJsonDriver, ProfiledVersionJsonFamily,
        merge_profile_with_behavior,
    },
    inspect::{find_library_version, inspect_driver_version_from_libraries},
};

const QUILT_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "quilt",
    name: "Quilt",
};

#[derive(Debug, Clone, Copy, Default)]
pub struct QuiltDriverFamily;

#[async_trait(?Send)]
impl ProfiledVersionJsonFamily for QuiltDriverFamily {
    type Source = QuiltSource;
    type Profile = ProfileJson;
    type RemoteResolver = super::prepared::QuiltRemoteResolver;

    fn descriptor(&self) -> DriverDescriptor {
        QUILT_DRIVER
    }

    fn default_source(&self) -> Result<Self::Source> {
        Ok(QuiltSource::default())
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver {
        super::prepared::QuiltRemoteResolver::new(
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
        merge_profile_with_behavior(&PASSTHROUGH_PROFILE_BEHAVIOR, base_metadata, profile)
    }

    fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &str,
        loader_version: &str,
    ) -> bool {
        let expected_id = format!("quilt-loader-{loader_version}-{game_version}");
        metadata.id != expected_id
            || metadata.inherits_from.as_deref() != Some(game_version)
            || inspect_driver_version(metadata).is_none_or(|installed| installed != loader_version)
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        inspect_driver_version_from_libraries(
            metadata,
            QUILT_DRIVER,
            &["org.quiltmc:quilt-loader:"],
        )
    }
}

pub type QuiltDriver = ProfiledVersionJsonDriver<QuiltDriverFamily>;

impl QuiltDriverFamily {
    pub fn new_driver(
        source: QuiltSource,
        vanilla_source: VanillaSource,
        downloader: Arc<ElementalDownloader>,
    ) -> QuiltDriver {
        ProfiledVersionJsonDriver::new(QuiltDriverFamily, source, vanilla_source, downloader)
    }

    pub fn new_driver_with_defaults() -> Result<QuiltDriver> {
        ProfiledVersionJsonDriver::with_defaults(QuiltDriverFamily)
    }
}

fn inspect_driver_version(metadata: &PistonMetaData) -> Option<String> {
    find_library_version(metadata, &["org.quiltmc:quilt-loader:"])
}
