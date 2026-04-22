use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::catalog::{
    Catalog, GameVersions, Release, ReleaseInfo, collect_single_game_releases,
    single_game_release_info,
};

use super::source::CleanroomSource;

pub const CLEANROOM_GAME_VERSION: &str = "1.12.2";
const CLEANROOM_RELEASE_DESCRIPTION: &str = "Cleanroom installer releases target Minecraft 1.12.2.";

#[derive(Debug, Clone)]
pub struct CleanroomCatalog {
    source: CleanroomSource,
}

#[derive(Debug, Clone)]
pub struct CleanroomRelease {
    pub loader: String,
    pub description: Option<String>,
}

impl Default for CleanroomCatalog {
    fn default() -> Self {
        Self::new(CleanroomSource::default())
    }
}

impl CleanroomCatalog {
    pub fn new(source: CleanroomSource) -> Self {
        Self { source }
    }

    pub fn source(&self) -> &CleanroomSource {
        &self.source
    }
}

#[async_trait]
impl Release for CleanroomRelease {
    async fn info(&self) -> ReleaseInfo {
        single_game_release_info(
            self.loader.clone(),
            CLEANROOM_GAME_VERSION.to_owned(),
            self.description.clone(),
        )
    }
}

#[async_trait]
impl Catalog for CleanroomCatalog {
    type Release = CleanroomRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>> {
        let metadata = self.source.maven_metadata().await?;
        Ok(collect_single_game_releases(
            metadata.versioning.versions.version,
            |version| {
                Some((
                    CLEANROOM_GAME_VERSION.to_owned(),
                    CleanroomRelease {
                        loader: version,
                        description: Some(CLEANROOM_RELEASE_DESCRIPTION.to_owned()),
                    },
                ))
            },
        ))
    }
}
