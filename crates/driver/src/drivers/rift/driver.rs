use anyhow::{Result, bail};
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::{fabric::ProfileJson, mojang::piston::PistonMetaData};

use crate::{
    driver::{DriverDescriptor, InstalledDriver},
    drivers::{rift::source::RiftSource, vanilla::source::VanillaSource},
    families::version_json::{
        PASSTHROUGH_PROFILE_BEHAVIOR, ProfiledVersionJsonDriver, ProfiledVersionJsonFamily,
        merge_profile_with_behavior,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
};

const RIFT_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "rift",
    name: "Rift",
};
const RIFT_PROFILE_ID: ProfileIdPattern = ProfileIdPattern::new("-rift-");
const RIFT_LIBRARY_MARKERS: LibraryPrefixSet =
    LibraryPrefixSet::new(&["org.dimdev:rift:", "com.github.Chocohead:Rift:"]);
const RIFT_VERSION_LIBRARIES: LibraryPrefixSet = LibraryPrefixSet::new(&["org.dimdev:rift:"]);
const RIFT_STALE_MARKERS: LibraryPrefixSet = LibraryPrefixSet::new(&["com.github.Chocohead:Rift:"]);
const RIFT_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    RIFT_DRIVER,
    RIFT_LIBRARY_MARKERS,
    RIFT_VERSION_LIBRARIES,
    RIFT_PROFILE_ID,
)
.with_stale_markers(RIFT_STALE_MARKERS);

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
            "rift",
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
        RIFT_IDENTITY.local_metadata_needs_refresh(metadata, game_version, loader_version.as_str())
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        RIFT_IDENTITY.inspect_installed(metadata)
    }
}

pub type RiftDriver = ProfiledVersionJsonDriver<RiftDriverFamily>;

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
