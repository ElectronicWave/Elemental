use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::catalog::{
    Catalog, GameVersions, Release, ReleaseInfo, collect_single_game_loader_releases,
    single_game_release_info,
};

use super::source::{FabricEndpointOverrides, FabricFlavor, FabricSource};

pub struct FabricCatalog {
    source: FabricSource,
}

pub struct FabricRelease {
    pub game_version: String,
    pub loader_version: String,
    pub description: Option<String>,
}

#[async_trait]
impl Release for FabricRelease {
    async fn info(&self) -> ReleaseInfo {
        single_game_release_info(
            self.loader_version.clone(),
            self.game_version.clone(),
            self.description.clone(),
        )
    }
}

impl FabricCatalog {
    pub fn new(source: FabricSource) -> Self {
        Self { source }
    }

    pub fn with_defaults() -> Self {
        Self::new(FabricSource::default())
    }

    pub fn for_flavor(flavor: FabricFlavor) -> Result<Self> {
        Ok(Self::new(FabricSource::for_flavor(flavor)?))
    }

    pub fn with_overrides(overrides: FabricEndpointOverrides) -> Result<Self> {
        Ok(Self::new(FabricSource::with_overrides(overrides)?))
    }
}

#[async_trait]
impl Catalog for FabricCatalog {
    type Release = FabricRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>> {
        let game_versions = self
            .source
            .game_versions()
            .await?
            .into_iter()
            .map(|game_version| game_version.version)
            .collect::<Vec<String>>();

        collect_single_game_loader_releases(
            game_versions,
            |game_version| async move { self.source.loader_versions(game_version.as_str()).await },
            |game_version, loader| FabricRelease {
                game_version: game_version.to_owned(),
                loader_version: loader.loader.version,
                description: Some(if loader.loader.stable {
                    "Stable".to_owned()
                } else {
                    "Unstable".to_owned()
                }),
            },
        )
        .await
    }
}
