use std::collections::HashSet;

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    catalog::{GameVersion, GameVersionCatalog, LoaderRelease, LoaderVersionCatalog},
    error::Result,
    loader_version::LoaderVersionId,
};

use super::source::RiftSource;

pub struct RiftCatalog {
    source: RiftSource,
}

impl RiftCatalog {
    pub fn new(source: RiftSource) -> Self {
        Self { source }
    }

    pub fn with_defaults() -> Self {
        Self::new(RiftSource::default())
    }
}

#[async_trait]
impl GameVersionCatalog for RiftCatalog {
    async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        let mut seen = HashSet::new();
        let mut game_versions = Vec::new();
        for release in self.source.releases().await? {
            let id = rift_game_version(release.loader_version.as_str());
            if seen.insert(id.clone()) {
                game_versions.push(GameVersion {
                    id,
                    description: None,
                    stream: None,
                });
            }
        }
        Ok(game_versions)
    }
}

#[async_trait]
impl LoaderVersionCatalog for RiftCatalog {
    async fn loader_versions(
        &self,
        game_version: &MinecraftVersionId,
    ) -> Result<Vec<LoaderRelease>> {
        let mut releases = Vec::new();
        for release in self.source.releases().await? {
            if &rift_game_version(release.loader_version.as_str()) != game_version {
                continue;
            }
            releases.push(
                LoaderRelease::new(
                    game_version.clone(),
                    LoaderVersionId::from(release.loader_version),
                )
                .with_published_at(release.published_at)
                .with_prerelease(Some(release.prerelease)),
            );
        }
        Ok(releases)
    }
}

fn rift_game_version(loader_version: &str) -> MinecraftVersionId {
    match loader_version {
        "1.0.4-106" => MinecraftVersionId::from("1.13.2"),
        _ => MinecraftVersionId::from("1.13"),
    }
}
