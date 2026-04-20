use anyhow::{Result, bail};
use async_trait::async_trait;
use elemental_core::storage::layout::Layout;
use std::collections::HashMap;

use crate::{
    catalog::{Catalog, GameVersions, Release, ReleaseInfo},
    driver::{Driver, DriverDescriptor, InstalledDriver},
    inspect::InstanceProbe,
};

#[derive(Default)]
pub struct NeoForgeDriver;

#[derive(Default)]
pub struct NeoForgeCatalog;

pub struct NeoForgeRelease {
    pub loader: String,
    pub game: String,
    pub description: Option<String>,
}

#[async_trait]
impl Release for NeoForgeRelease {
    async fn info(&self) -> ReleaseInfo {
        ReleaseInfo {
            name: self.loader.clone(),
            game_versions: GameVersions::Single(self.game.clone()),
            description: self.description.clone(),
        }
    }
}

#[async_trait]
impl Catalog for NeoForgeCatalog {
    type Release = NeoForgeRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>> {
        bail!("NeoForge catalog is not implemented yet");
    }
}

#[async_trait]
impl<L: Layout, VL: Layout> Driver<L, VL> for NeoForgeDriver {
    fn descriptor(&self) -> DriverDescriptor {
        DriverDescriptor {
            id: "neoforge",
            name: "NeoForge",
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
            .find(|name| {
                name.starts_with("net.neoforged:forge:")
                    || name.starts_with("net.neoforged:neoforge:")
            });

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
