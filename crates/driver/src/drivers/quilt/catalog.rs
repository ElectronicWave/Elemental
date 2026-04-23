use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::catalog::{
    Catalog, Release, ReleaseInfo, collect_single_game_loader_releases, single_game_release_info,
};
use crate::loader_version::LoaderVersionId;

use super::source::QuiltSource;

pub struct QuiltCatalog {
    source: QuiltSource,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QuiltRelease {
    pub game_version: MinecraftVersionId,
    pub loader_version: LoaderVersionId,
}

#[async_trait]
impl Release for QuiltRelease {
    async fn info(&self) -> ReleaseInfo {
        single_game_release_info(
            self.loader_version.to_string(),
            self.game_version.clone(),
            None,
        )
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

    async fn releases(&self) -> Result<HashMap<MinecraftVersionId, Vec<Self::Release>>> {
        let game_versions = self
            .source
            .game_versions()
            .await?
            .into_iter()
            .map(|game_version| MinecraftVersionId::from(game_version.version))
            .collect::<Vec<MinecraftVersionId>>();

        collect_single_game_loader_releases(
            game_versions,
            |game_version| async move { self.source.loader_versions(game_version.as_str()).await },
            |game_version, loader| QuiltRelease {
                game_version: game_version.clone(),
                loader_version: LoaderVersionId::from(loader.loader.version),
            },
        )
        .await
    }
}
