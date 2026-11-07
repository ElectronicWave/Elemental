use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::fabric::ProfileJson;

use crate::{
    descriptors::RIFT_DRIVER,
    driver::DriverDescriptor,
    drivers::{rift::source::RiftSource, vanilla::source::VanillaSource},
    error::{Error, Result},
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
#[derive(Debug, Clone, Copy, Default)]
pub struct RiftDriverFamily;

#[async_trait(?Send)]
impl ProfiledVersionJsonFamily for RiftDriverFamily {
    type Source = RiftSource;
    type RemoteResolver = super::prepared::RiftRemoteResolver;

    fn descriptor(&self) -> DriverDescriptor {
        RIFT_DRIVER
    }

    fn identity(&self) -> Option<ProfiledDriverIdentity> {
        Some(RIFT_IDENTITY)
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver {
        crate::families::version_json::VanillaFallbackRemoteResolver::new(
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

    Err(Error::RiftGameVersionMismatch {
        loader_version: loader_version.as_str().to_owned(),
        actual: profile.inherits_from.clone(),
        requested: requested_game_version.as_str().to_owned(),
    })
}
