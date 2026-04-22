use anyhow::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::{fabric::ProfileJson, mojang::piston::PistonMetaData};

use crate::{
    driver::{DriverDescriptor, InstalledDriver},
    drivers::{liteloader::source::LiteLoaderSource, vanilla::source::VanillaSource},
    families::version_json::{
        PASSTHROUGH_PROFILE_BEHAVIOR, ProfiledVersionJsonDriver, ProfiledVersionJsonFamily,
        merge_profile_with_behavior,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
};

const LITELOADER_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "liteloader",
    name: "LiteLoader",
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
        LITELOADER_IDENTITY.local_metadata_needs_refresh(
            metadata,
            game_version,
            loader_version.as_str(),
        )
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        LITELOADER_IDENTITY.inspect_installed(metadata)
    }
}

pub type LiteLoaderDriver = ProfiledVersionJsonDriver<LiteLoaderDriverFamily>;
