use anyhow::Result;
use async_trait::async_trait;
use elemental_core::storage::layout::Layout;
use elemental_schema::fabric::{GameVersion, LoaderVersion};
use std::collections::HashMap;

use crate::{
    catalog::{Catalog, GameVersions, Release, ReleaseInfo},
    driver::{Driver, DriverDescriptor, InstalledDriver},
    inspect::InstanceProbe,
};

#[derive(Clone)]
pub enum FabricFlavor {
    Fabric,
    LegacyFabric,
    Babric,
    Custom(String),
}

#[derive(Default)]
pub struct FabricDriver;

pub struct FabricCatalog {
    pub flavor: FabricFlavor,
}

pub struct FabricRelease {
    pub loader: String,
    pub flavor: FabricFlavor,
    pub game_versions: GameVersions,
    pub description: Option<String>,
}

impl FabricFlavor {
    pub fn meta_url(&self) -> String {
        match self {
            FabricFlavor::Fabric => "https://meta.fabricmc.net".to_owned(),
            FabricFlavor::LegacyFabric => "https://meta.legacyfabric.net".to_owned(),
            FabricFlavor::Babric => "https://meta.babric.glass-launcher.net".to_owned(),
            FabricFlavor::Custom(url) => url.to_owned(),
        }
    }
}

impl Default for FabricCatalog {
    fn default() -> Self {
        Self {
            flavor: FabricFlavor::Fabric,
        }
    }
}

impl FabricCatalog {
    pub fn legacy() -> Self {
        Self {
            flavor: FabricFlavor::LegacyFabric,
        }
    }

    pub fn babric() -> Self {
        Self {
            flavor: FabricFlavor::Babric,
        }
    }

    pub fn custom(url: String) -> Self {
        Self {
            flavor: FabricFlavor::Custom(url),
        }
    }
}

#[async_trait]
impl Release for FabricRelease {
    async fn install(&self) -> Result<()> {
        todo!()
    }

    async fn uninstall(&self) -> Result<()> {
        todo!()
    }

    async fn info(&self) -> ReleaseInfo {
        ReleaseInfo {
            name: self.loader.clone(),
            game_versions: self.game_versions.clone(),
            description: self.description.clone(),
        }
    }
}

#[async_trait]
impl Catalog for FabricCatalog {
    type Release = FabricRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>> {
        let mut data = HashMap::new();
        let raw = reqwest::get(format!("{}/v2/versions/loader", self.flavor.meta_url()))
            .await?
            .text()
            .await?;
        let body: Vec<LoaderVersion> = serde_json::from_str(&raw)?;
        let game_body: Vec<GameVersion> = serde_json::from_str(
            &reqwest::get(format!("{}/v2/versions/game", self.flavor.meta_url()))
                .await?
                .text()
                .await?,
        )?;
        let game_versions = GameVersions::Multi(
            game_body
                .into_iter()
                .map(|game| game.version)
                .collect::<Vec<String>>(),
        );

        for loader in body {
            data.entry(game_versions.clone())
                .or_insert(Vec::new())
                .push(FabricRelease {
                    loader: loader.version.clone(),
                    flavor: self.flavor.clone(),
                    game_versions: game_versions.clone(),
                    description: Some(if loader.stable {
                        "Stable".to_owned()
                    } else {
                        "Unstable".to_owned()
                    }),
                });
        }
        Ok(data)
    }
}

#[async_trait]
impl<L: Layout, VL: Layout> Driver<L, VL> for FabricDriver {
    fn descriptor(&self) -> DriverDescriptor {
        DriverDescriptor {
            id: "fabric",
            name: "Fabric",
        }
    }

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };
        let library_name = metadata
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .find(|name| name.starts_with("net.fabricmc:fabric-loader:"));

        let Some(library_name) = library_name else {
            return Ok(None);
        };

        let driver_version = library_name.split(':').nth(2).map(ToOwned::to_owned);

        Ok(Some(InstalledDriver {
            driver: <Self as Driver<L, VL>>::descriptor(self),
            driver_version,
            game_version: Some(metadata.id.clone()),
            description: Some(metadata.release_type.clone()),
        }))
    }
}
