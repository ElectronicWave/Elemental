use anyhow::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::fabric::ProfileJson;

use crate::{
    driver::DriverDescriptor,
    drivers::liteloader::source::LiteLoaderSource,
    families::version_json::{PassthroughProfiledVersionJsonFamily, ProfiledVersionJsonDriver},
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
impl PassthroughProfiledVersionJsonFamily for LiteLoaderDriverFamily {
    type Source = LiteLoaderSource;
    type Endpoints = super::source::LiteLoaderEndpoints;

    const DRIVER: DriverDescriptor = LITELOADER_DRIVER;
    const FAMILY_NAME: &'static str = "liteloader";
    const IDENTITY: ProfiledDriverIdentity = LITELOADER_IDENTITY;

    fn source_endpoints(source: &Self::Source) -> &Self::Endpoints {
        source.endpoints()
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
