use crate::error::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::fabric::ProfileJson;

use crate::{
    descriptors::LITELOADER_DRIVER,
    driver::DriverDescriptor,
    drivers::{liteloader::source::LiteLoaderSource, vanilla::source::VanillaSource},
    families::version_json::{ProfiledVersionJsonDriver, ProfiledVersionJsonFamily},
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
};

const LITELOADER_PROFILE_ID: ProfileIdPattern = ProfileIdPattern::new("-liteloader-");
const LITELOADER_LIBRARIES: LibraryPrefixSet = LibraryPrefixSet::new(&["com.mumfrey:liteloader:"]);
const LITELOADER_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    LITELOADER_DRIVER,
    LITELOADER_LIBRARIES,
    LITELOADER_LIBRARIES,
    LITELOADER_PROFILE_ID,
);
#[derive(Debug, Clone, Copy, Default)]
pub struct LiteLoaderDriverFamily;

#[async_trait(?Send)]
impl ProfiledVersionJsonFamily for LiteLoaderDriverFamily {
    type Source = LiteLoaderSource;
    type RemoteResolver = super::prepared::LiteLoaderRemoteResolver;

    fn descriptor(&self) -> DriverDescriptor {
        LITELOADER_DRIVER
    }

    fn identity(&self) -> Option<ProfiledDriverIdentity> {
        Some(LITELOADER_IDENTITY)
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver {
        crate::families::version_json::VanillaFallbackRemoteResolver::new(
            "liteloader",
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
        source
            .profile_json(game_version.as_str(), loader_version.as_str())
            .await
    }
}

pub type LiteLoaderDriver = ProfiledVersionJsonDriver<LiteLoaderDriverFamily>;
