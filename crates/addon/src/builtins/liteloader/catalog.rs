use std::collections::HashSet;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_driver::{
    catalog::{GameVersion, LoaderRelease},
    loader_version::LoaderVersionId,
};

use crate::error::Result;

use super::source::LiteLoaderSource;

#[derive(Debug, Clone)]
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

    pub async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        let mut seen = HashSet::new();
        Ok(self
            .source
            .releases()
            .await?
            .into_iter()
            .filter_map(|release| {
                let id = MinecraftVersionId::from(release.game_version);
                seen.insert(id.clone())
                    .then_some(GameVersion { id, channel: None })
            })
            .collect())
    }

    pub async fn loader_versions(
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
                .with_channel(Some(release.channel))
            })
            .collect())
    }
}
