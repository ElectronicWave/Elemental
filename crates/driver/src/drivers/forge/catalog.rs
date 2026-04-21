use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::catalog::{
    Catalog, GameVersions, Release, ReleaseInfo, push_single_game_release, single_game_release_info,
};

use super::source::ForgeSource;

#[derive(Debug, Clone)]
pub struct ForgeCatalog {
    source: ForgeSource,
}

#[derive(Debug, Clone)]
pub struct ForgeRelease {
    pub loader: String,
    pub game: String,
    pub description: Option<String>,
}

impl Default for ForgeCatalog {
    fn default() -> Self {
        Self::new(ForgeSource::default())
    }
}

impl ForgeCatalog {
    pub fn new(source: ForgeSource) -> Self {
        Self { source }
    }

    pub fn source(&self) -> &ForgeSource {
        &self.source
    }
}

#[async_trait]
impl Release for ForgeRelease {
    async fn info(&self) -> ReleaseInfo {
        single_game_release_info(
            self.loader.clone(),
            self.game.clone(),
            self.description.clone(),
        )
    }
}

#[async_trait]
impl Catalog for ForgeCatalog {
    type Release = ForgeRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>> {
        let mut releases = HashMap::new();
        let metadata = self.source.maven_metadata().await?;

        for version in metadata.versioning.versions.version {
            let Some((game_version, loader_version)) = version.split_once('-') else {
                continue;
            };

            push_single_game_release(
                &mut releases,
                game_version.to_owned(),
                ForgeRelease {
                    loader: loader_version.to_owned(),
                    game: game_version.to_owned(),
                    description: None,
                },
            );
        }

        Ok(releases)
    }
}
