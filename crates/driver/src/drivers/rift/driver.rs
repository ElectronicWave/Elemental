use anyhow::{Result, bail};
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::fabric::ProfileJson;

use crate::{
    driver::DriverDescriptor,
    drivers::rift::source::RiftSource,
    families::version_json::{PassthroughProfiledVersionJsonFamily, ProfiledVersionJsonDriver},
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
impl PassthroughProfiledVersionJsonFamily for RiftDriverFamily {
    type Source = RiftSource;
    type Endpoints = super::source::RiftEndpoints;

    const DRIVER: DriverDescriptor = RIFT_DRIVER;
    const FAMILY_NAME: &'static str = "rift";
    const IDENTITY: ProfiledDriverIdentity = RIFT_IDENTITY;

    fn source_endpoints(source: &Self::Source) -> &Self::Endpoints {
        source.endpoints()
    }

    async fn profile(
        &self,
        source: &Self::Source,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<ProfileJson> {
        let profile = source.profile_json(loader_version.as_str()).await?;
        validate_requested_game_version(&profile, game_version, loader_version)?;
        Ok(profile)
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
