use anyhow::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::{fabric::ProfileJson, mojang::piston::PistonMetaData};

use crate::{
    descriptors::LITELOADER_DRIVER,
    driver::{DriverDescriptor, InstalledDriver},
    drivers::{liteloader::source::LiteLoaderSource, vanilla::source::VanillaSource},
    families::version_json::direct_profiled::DirectProfiledVersionJsonDefaults,
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
const LITELOADER_DEFAULTS: DirectProfiledVersionJsonDefaults =
    DirectProfiledVersionJsonDefaults::new(LITELOADER_DRIVER, "liteloader", LITELOADER_IDENTITY);

#[derive(Debug, Clone, Copy, Default)]
pub struct LiteLoaderDriverFamily;

#[async_trait(?Send)]
impl ProfiledVersionJsonFamily for LiteLoaderDriverFamily {
    type Source = LiteLoaderSource;
    type Profile = ProfileJson;
    type RemoteResolver = super::prepared::LiteLoaderRemoteResolver;

    fn descriptor(&self) -> DriverDescriptor {
        LITELOADER_DEFAULTS.descriptor()
    }

    fn default_source(&self) -> Result<Self::Source> {
        LITELOADER_DEFAULTS.default_source()
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver {
        LITELOADER_DEFAULTS.remote_resolver(vanilla_source, source.endpoints())
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
        LITELOADER_DEFAULTS.merge_profile(base_metadata, profile)
    }

    fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool {
        LITELOADER_DEFAULTS.local_metadata_needs_refresh(metadata, game_version, loader_version)
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        LITELOADER_DEFAULTS.inspect_installed(metadata)
    }
}

pub type LiteLoaderDriver = ProfiledVersionJsonDriver<LiteLoaderDriverFamily>;
