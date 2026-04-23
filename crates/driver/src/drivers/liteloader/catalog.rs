use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::catalog::{
    Catalog, Release, ReleaseInfo, push_single_game_release, single_game_release_info,
};
use crate::loader_version::LoaderVersionId;

use super::source::LiteLoaderSource;

pub struct LiteLoaderCatalog {
    source: LiteLoaderSource,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LiteLoaderCatalogRelease {
    pub game_version: MinecraftVersionId,
    pub loader_version: LoaderVersionId,
    pub stream: String,
}

#[async_trait]
impl Release for LiteLoaderCatalogRelease {
    async fn info(&self) -> ReleaseInfo {
        single_game_release_info(
            self.loader_version.to_string(),
            self.game_version.clone(),
            Some(self.stream.to_ascii_lowercase()),
        )
    }
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
impl Catalog for LiteLoaderCatalog {
    type Release = LiteLoaderCatalogRelease;

    async fn releases(&self) -> Result<HashMap<MinecraftVersionId, Vec<Self::Release>>> {
        let mut releases = HashMap::new();

        for release in self.source.releases().await? {
            let game_version = MinecraftVersionId::from(release.game_version);
            push_single_game_release(
                &mut releases,
                game_version.clone(),
                LiteLoaderCatalogRelease {
                    game_version,
                    loader_version: LoaderVersionId::from(release.loader_version),
                    stream: release.stream,
                },
            );
        }

        Ok(releases)
    }
}
