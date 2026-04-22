use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_infra::downloader::core::ElementalDownloader;
use elemental_schema::{fabric::ProfileJson, mojang::piston::PistonMetaData};

use crate::{
    driver::{DriverDescriptor, InstalledDriver},
    drivers::{liteloader::source::LiteLoaderSource, vanilla::source::VanillaSource},
    families::version_json::{
        PASSTHROUGH_PROFILE_BEHAVIOR, ProfiledVersionJsonDriver, ProfiledVersionJsonFamily,
        merge_profile_with_behavior,
    },
    inspect::{
        find_library_version, installed_version_json_driver, metadata_contains_library_prefix,
    },
    loader_version::LoaderVersionId,
};

const LITELOADER_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "liteloader",
    name: "LiteLoader",
};

#[derive(Debug, Clone, Copy, Default)]
pub struct LiteLoaderDriverFamily;

#[async_trait(?Send)]
impl ProfiledVersionJsonFamily for LiteLoaderDriverFamily {
    type Source = LiteLoaderSource;
    type Profile = ProfileJson;
    type RemoteResolver = super::prepared::LiteLoaderRemoteResolver;

    fn descriptor(&self) -> DriverDescriptor {
        LITELOADER_DRIVER
    }

    fn default_source(&self) -> Result<Self::Source> {
        Ok(LiteLoaderSource::default())
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver {
        super::prepared::LiteLoaderRemoteResolver::new(
            vanilla_source.endpoints().clone(),
            source.endpoints().clone(),
        )
    }

    async fn profile(
        &self,
        source: &Self::Source,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<Self::Profile> {
        source
            .profile_json(game_version.as_str(), loader_version.as_str())
            .await
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
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool {
        let expected_id = build_profile_id(game_version.as_str(), loader_version.as_str());
        metadata.id != expected_id
            || metadata.inherits_from.as_deref() != Some(game_version.as_str())
            || inspect_driver_version(metadata)
                .is_none_or(|installed| installed != loader_version.as_str())
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        if !is_liteloader_metadata(metadata) {
            return None;
        }

        Some(installed_version_json_driver(
            metadata,
            LITELOADER_DRIVER,
            inspect_driver_version(metadata),
        ))
    }
}

pub type LiteLoaderDriver = ProfiledVersionJsonDriver<LiteLoaderDriverFamily>;

impl LiteLoaderDriverFamily {
    pub fn new_driver(
        source: LiteLoaderSource,
        vanilla_source: VanillaSource,
        downloader: Arc<ElementalDownloader>,
    ) -> LiteLoaderDriver {
        ProfiledVersionJsonDriver::new(LiteLoaderDriverFamily, source, vanilla_source, downloader)
    }

    pub fn new_driver_with_defaults() -> Result<LiteLoaderDriver> {
        ProfiledVersionJsonDriver::with_defaults(LiteLoaderDriverFamily)
    }
}

fn build_profile_id(game_version: &str, loader_version: &str) -> String {
    format!("{game_version}-liteloader-{loader_version}")
}

fn is_liteloader_metadata(metadata: &PistonMetaData) -> bool {
    metadata_contains_library_prefix(metadata, &["com.mumfrey:liteloader:"])
        || extract_loader_version_from_profile_id(metadata.id.as_str()).is_some()
}

fn inspect_driver_version(metadata: &PistonMetaData) -> Option<String> {
    find_library_version(metadata, &["com.mumfrey:liteloader:"])
        .or_else(|| extract_loader_version_from_profile_id(metadata.id.as_str()))
}

fn extract_loader_version_from_profile_id(metadata_id: &str) -> Option<String> {
    metadata_id
        .split_once("-liteloader-")
        .map(|(_, loader_version)| loader_version.to_owned())
}
