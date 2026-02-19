use crate::base::{ModLoader, ModLoaderVersion, ModLoaderVersionInfo, Version};
use anyhow::Result;
use async_trait::async_trait;
use elemental_core::storage::{layout::Layout, version::VersionStorage};
use serde::Deserialize;
use std::collections::HashMap;

/// Fabric: https://meta.fabricmc.net/ - 1.14 ~ 26.1(Latest)
/// LegacyFabric: https://meta.legacyfabric.net/ - 1.13.2 ~ 1.3.2
/// Babric: https://meta.babric.glass-launcher.net/ - b1.7.3

#[derive(Clone)]
pub enum Fabric {
    Fabric,
    LegacyFabric,
    Babric,
    Custom(String),
}

impl Fabric {
    pub fn get_meta_url(&self) -> String {
        match self {
            Fabric::Fabric => "https://meta.fabricmc.net".to_owned(),
            Fabric::LegacyFabric => "https://meta.legacyfabric.net".to_owned(),
            Fabric::Babric => "https://meta.babric.glass-launcher.net".to_owned(),
            Fabric::Custom(url) => url.to_owned(),
        }
    }
}

pub struct FabricLike {
    pub fabric: Fabric,
}

impl Default for FabricLike {
    fn default() -> Self {
        Self {
            fabric: Fabric::Fabric,
        }
    }
}

impl FabricLike {
    pub fn legacy() -> Self {
        Self {
            fabric: Fabric::LegacyFabric,
        }
    }

    pub fn babric() -> Self {
        Self {
            fabric: Fabric::Babric,
        }
    }

    pub fn new(url: String) -> Self {
        Self {
            fabric: Fabric::Custom(url),
        }
    }
}

pub struct FabricLoaderVersion {
    pub loader: String,
    pub fabric: Fabric,
    pub game: Version,
    pub description: Option<String>,
}

#[async_trait]
impl ModLoaderVersion for FabricLoaderVersion {
    async fn install(&self) -> anyhow::Result<()> {
        todo!()
    }

    async fn uninstall(&self) -> anyhow::Result<()> {
        todo!()
    }

    async fn info(&self) -> ModLoaderVersionInfo {
        ModLoaderVersionInfo {
            name: self.loader.clone(),
            version: self.game.clone(),
            description: self.description.clone(),
        }
    }
}

#[async_trait]
impl ModLoader for FabricLike {
    type T = FabricLoaderVersion;

    async fn versions(&self) -> Result<HashMap<Version, Vec<Self::T>>> {
        let mut data = HashMap::new();
        let raw = reqwest::get(format!("{}/v2/versions/loader", self.fabric.get_meta_url()))
            .await?
            .text()
            .await?;
        let body: Vec<LoaderVersion> = serde_json::from_str(&raw)?;
        let game_body: Vec<GameVersion> = serde_json::from_str(
            &reqwest::get(format!("{}/v2/versions/game", self.fabric.get_meta_url()))
                .await?
                .text()
                .await?,
        )?;
        let mut game_version = Vec::new();
        for game in game_body {
            game_version.push(game.version.clone());
        }
        for loader in body {
            let version = Version::MULTI(game_version.clone());
            data.entry(version.clone())
                .or_insert(Vec::new())
                .push(FabricLoaderVersion {
                    loader: loader.version.clone(),
                    fabric: self.fabric.clone(),
                    game: version,
                    description: if loader.stable {
                        Some("Stable".to_owned())
                    } else {
                        Some("Unstable".to_owned())
                    },
                });
        }
        Ok(data)
    }

    async fn installed<L: Layout, VL: Layout>(
        &self,
        version: VersionStorage<L, VL>,
    ) -> Result<Option<FabricLoaderVersion>> {
        todo!()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct LoaderVersion {
    pub separator: String,
    pub build: i32,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GameVersion {
    pub version: String,
    pub stable: bool,
}
