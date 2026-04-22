use std::sync::Arc;

use anyhow::{Result, bail};
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_infra::downloader::core::ElementalDownloader;
use elemental_schema::{fabric::ProfileJson, mojang::piston::PistonMetaData};

use crate::{
    driver::{DriverDescriptor, InstalledDriver},
    drivers::{
        rift::source::{LEGACY_RIFT_LIBRARY_PREFIX, NORMALIZED_RIFT_LIBRARY_PREFIX, RiftSource},
        vanilla::source::VanillaSource,
    },
    families::version_json::{
        PASSTHROUGH_PROFILE_BEHAVIOR, ProfiledVersionJsonDriver, ProfiledVersionJsonFamily,
        merge_profile_with_behavior,
    },
    inspect::{
        find_library_version, installed_version_json_driver, metadata_contains_library_prefix,
    },
    loader_version::LoaderVersionId,
};

const RIFT_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "rift",
    name: "Rift",
};

#[derive(Debug, Clone, Copy, Default)]
pub struct RiftDriverFamily;

#[async_trait(?Send)]
impl ProfiledVersionJsonFamily for RiftDriverFamily {
    type Source = RiftSource;
    type Profile = ProfileJson;
    type RemoteResolver = super::prepared::RiftRemoteResolver;

    fn descriptor(&self) -> DriverDescriptor {
        RIFT_DRIVER
    }

    fn default_source(&self) -> Result<Self::Source> {
        Ok(RiftSource::default())
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver {
        super::prepared::RiftRemoteResolver::new(
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
        let profile = source.profile_json(loader_version.as_str()).await?;
        validate_requested_game_version(&profile, game_version, loader_version)?;
        Ok(profile)
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
            || metadata_contains_library_prefix(metadata, &[LEGACY_RIFT_LIBRARY_PREFIX])
            || inspect_driver_version(metadata)
                .is_none_or(|installed| installed != loader_version.as_str())
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        if !is_rift_metadata(metadata) {
            return None;
        }

        Some(installed_version_json_driver(
            metadata,
            RIFT_DRIVER,
            inspect_driver_version(metadata),
        ))
    }
}

pub type RiftDriver = ProfiledVersionJsonDriver<RiftDriverFamily>;

impl RiftDriverFamily {
    pub fn new_driver(
        source: RiftSource,
        vanilla_source: VanillaSource,
        downloader: Arc<ElementalDownloader>,
    ) -> RiftDriver {
        ProfiledVersionJsonDriver::new(RiftDriverFamily, source, vanilla_source, downloader)
    }

    pub fn new_driver_with_defaults() -> Result<RiftDriver> {
        ProfiledVersionJsonDriver::with_defaults(RiftDriverFamily)
    }
}

fn validate_requested_game_version(
    profile: &ProfileJson,
    requested_game_version: &MinecraftVersionId,
    loader_version: &LoaderVersionId,
) -> Result<()> {
    if profile.inherits_from == requested_game_version.as_str() {
        return Ok(());
    }

    bail!(
        "Rift loader '{}' targets Minecraft '{}' but '{}' was requested",
        loader_version.as_str(),
        profile.inherits_from,
        requested_game_version.as_str(),
    )
}

fn build_profile_id(game_version: &str, loader_version: &str) -> String {
    format!("{game_version}-rift-{loader_version}")
}

fn is_rift_metadata(metadata: &PistonMetaData) -> bool {
    metadata_contains_library_prefix(
        metadata,
        &[NORMALIZED_RIFT_LIBRARY_PREFIX, LEGACY_RIFT_LIBRARY_PREFIX],
    ) || extract_loader_version_from_profile_id(metadata.id.as_str()).is_some()
}

fn inspect_driver_version(metadata: &PistonMetaData) -> Option<String> {
    find_library_version(metadata, &[NORMALIZED_RIFT_LIBRARY_PREFIX])
        .or_else(|| extract_loader_version_from_profile_id(metadata.id.as_str()))
}

fn extract_loader_version_from_profile_id(metadata_id: &str) -> Option<String> {
    metadata_id
        .split_once("-rift-")
        .map(|(_, loader_version)| loader_version.to_owned())
}
