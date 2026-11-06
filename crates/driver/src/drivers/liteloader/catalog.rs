use std::collections::HashSet;

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    catalog::{GameVersion, GameVersionCatalog, LoaderRelease, LoaderVersionCatalog},
    error::Result,
    loader_version::LoaderVersionId,
};

use super::source::LiteLoaderSource;

pub struct LiteLoaderCatalog {
    source: LiteLoaderSource,
}

impl LiteLoaderCatalog {
    pub fn new(source: LiteLoaderSource) -> Self {
        Self { source }
    }

    pub fn with_defaults() -> Self {
        Self::new(LiteLoaderSource::default())
    }
}

#[async_trait]
impl GameVersionCatalog for LiteLoaderCatalog {
    async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        let mut seen = HashSet::new();
        Ok(self
            .source
            .releases()
            .await?
            .into_iter()
            .filter_map(|release| {
                let id = MinecraftVersionId::from(release.game_version);
                seen.insert(id.clone()).then_some(GameVersion {
                    id,
                    description: None,
                    stream: None,
                })
            })
            .collect())
    }
}

#[async_trait]
impl LoaderVersionCatalog for LiteLoaderCatalog {
    async fn loader_versions(
        &self,
        game_version: &MinecraftVersionId,
    ) -> Result<Vec<LoaderRelease>> {
        Ok(self
            .source
            .releases()
            .await?
            .into_iter()
            .filter(|release| release.game_version == game_version.as_str())
            .map(|release| {
                LoaderRelease::new(
                    game_version.clone(),
                    LoaderVersionId::from(release.loader_version),
                )
                .with_stream(Some(release.stream))
            })
            .collect())
    }
}
