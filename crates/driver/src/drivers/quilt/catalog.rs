use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::catalog::{Catalog, GameVersions, Release, ReleaseInfo};

use super::source::QuiltSource;

pub struct QuiltCatalog {
    source: QuiltSource,
}

pub struct QuiltRelease {
    pub game_version: String,
    pub loader_version: String,
}

#[async_trait]
impl Release for QuiltRelease {
    async fn info(&self) -> ReleaseInfo {
        ReleaseInfo {
            name: self.loader_version.clone(),
            game_versions: GameVersions::Single(self.game_version.clone()),
            description: None,
        }
    }
}

impl QuiltCatalog {
    pub fn new(source: QuiltSource) -> Self {
        Self { source }
    }

    pub fn with_defaults() -> Self {
        Self::new(QuiltSource::default())
    }
}

#[async_trait]
impl Catalog for QuiltCatalog {
    type Release = QuiltRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>> {
        let mut releases = HashMap::new();

        for game_version in self.source.game_versions().await? {
            let game_releases = self
                .source
                .loader_versions(game_version.version.as_str())
                .await?
                .into_iter()
                .map(|loader| QuiltRelease {
                    game_version: game_version.version.clone(),
                    loader_version: loader.loader.version,
                })
                .collect::<Vec<QuiltRelease>>();

            releases.insert(GameVersions::Single(game_version.version), game_releases);
        }

        Ok(releases)
    }
}
