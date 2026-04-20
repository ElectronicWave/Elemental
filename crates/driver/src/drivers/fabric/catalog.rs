use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::catalog::{Catalog, GameVersions, Release, ReleaseInfo};

use super::source::{FabricFlavor, FabricSource};

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
        ReleaseInfo {
            name: self.loader_version.clone(),
            game_versions: GameVersions::Single(self.game_version.clone()),
            description: self.description.clone(),
        }
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
}

#[async_trait]
impl Catalog for FabricCatalog {
    type Release = FabricRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>> {
        let mut releases = HashMap::new();

        for game_version in self.source.game_versions().await? {
            let mut game_releases = Vec::new();
            for loader in self
                .source
                .loader_versions(game_version.version.as_str())
                .await?
            {
                game_releases.push(FabricRelease {
                    game_version: game_version.version.clone(),
                    loader_version: loader.loader.version,
                    description: Some(if loader.loader.stable {
                        "Stable".to_owned()
                    } else {
                        "Unstable".to_owned()
                    }),
                });
            }

            releases.insert(GameVersions::Single(game_version.version), game_releases);
        }

        Ok(releases)
    }
}
