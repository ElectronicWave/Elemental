use anyhow::{Result, bail};
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::{fabric::ProfileJson, mojang::piston::PistonMetaData};

use crate::{
    descriptors::RIFT_DRIVER,
    driver::{DriverDescriptor, InstalledDriver},
    drivers::{rift::source::RiftSource, vanilla::source::VanillaSource},
    families::version_json::direct_profiled::DirectProfiledVersionJsonDefaults,
    families::version_json::{ProfiledVersionJsonDriver, ProfiledVersionJsonFamily},
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
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
const RIFT_DEFAULTS: DirectProfiledVersionJsonDefaults =
    DirectProfiledVersionJsonDefaults::new(RIFT_DRIVER, "rift", RIFT_IDENTITY);

#[derive(Debug, Clone, Copy, Default)]
pub struct RiftDriverFamily;

#[async_trait(?Send)]
impl ProfiledVersionJsonFamily for RiftDriverFamily {
    type Source = RiftSource;
    type Profile = ProfileJson;
    type RemoteResolver = super::prepared::RiftRemoteResolver;

    fn descriptor(&self) -> DriverDescriptor {
        RIFT_DEFAULTS.descriptor()
    }

    fn default_source(&self) -> Result<Self::Source> {
        RIFT_DEFAULTS.default_source()
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver {
        RIFT_DEFAULTS.remote_resolver(vanilla_source, source.endpoints())
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
        RIFT_DEFAULTS.merge_profile(base_metadata, profile)
    }

    fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool {
        RIFT_DEFAULTS.local_metadata_needs_refresh(metadata, game_version, loader_version)
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        RIFT_DEFAULTS.inspect_installed(metadata)
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
