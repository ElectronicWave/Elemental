use anyhow::Result;
use async_trait::async_trait;
use elemental_core::storage::layout::Layout;
use elemental_schema::forge::MavenMetadataBody;
use quick_xml::de::from_str;
use std::collections::HashMap;

use crate::{
    catalog::{Catalog, GameVersions, Release, ReleaseInfo},
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::version_json::resource::Resource,
    inspect::VersionProbe,
};

pub struct ForgeCatalog {
    pub files: String,
    pub maven: String,
}

#[derive(Default)]
pub struct ForgeDriver;

pub struct ForgeRelease {
    pub loader: String,
    pub game: String,
    pub description: Option<String>,
}

impl Default for ForgeCatalog {
    fn default() -> Self {
        Self {
            files: "files.minecraftforge.net".to_owned(),
            maven: "maven.minecraftforge.net".to_owned(),
        }
    }
}

#[async_trait]
impl Release for ForgeRelease {
    async fn install(&self) -> Result<()> {
        todo!()
    }

    async fn uninstall(&self) -> Result<()> {
        todo!()
    }

    async fn info(&self) -> ReleaseInfo {
        ReleaseInfo {
            name: self.loader.clone(),
            game_versions: GameVersions::Single(self.game.clone()),
            description: self.description.clone(),
        }
    }
}

#[async_trait]
impl Catalog for ForgeCatalog {
    type Release = ForgeRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<ForgeRelease>>> {
        let mut data = HashMap::new();
        let raw = reqwest::get(format!(
            "https://{}/net/minecraftforge/forge/maven-metadata.xml",
            self.maven,
        ))
        .await?
        .text()
        .await?;
        let body: MavenMetadataBody = from_str(&raw)?;
        for version in body.versioning.versions.version {
            if let Some((game_version, loader_version)) = version.split_once("-") {
                data.entry(GameVersions::Single(game_version.to_owned()))
                    .or_insert(Vec::new())
                    .push(ForgeRelease {
                        loader: loader_version.to_owned(),
                        game: game_version.to_owned(),
                        description: None,
                    });
            }
        }
        Ok(data)
    }
}

#[async_trait]
impl<L: Layout<Resource = Resource>, VL: Layout> Driver<L, VL> for ForgeDriver {
    fn descriptor(&self) -> DriverDescriptor {
        DriverDescriptor {
            id: "forge",
            name: "Forge",
        }
    }

    async fn inspect(&self, probe: &VersionProbe<L, VL>) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };
        let library_name = metadata
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .find(|name| name.starts_with("net.minecraftforge:forge:"));

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
