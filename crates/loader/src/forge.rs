// https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml
// https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json

use crate::base::{ModLoader, ModLoaderVersion, ModLoaderVersionInfo};
use anyhow::Result;
use async_trait::async_trait;
use elemental_core::storage::version::VersionStorage;
use quick_xml::de::from_str;
use serde::Deserialize;
use std::collections::HashMap;

pub struct ForgeModLoader {
    pub files: String,
    pub maven: String,
}

impl Default for ForgeModLoader {
    fn default() -> Self {
        Self {
            files: "files.minecraftforge.net".to_owned(),
            maven: "maven.minecraftforge.net".to_owned(),
        }
    }
}

pub struct ForgeModLoaderVersion {
    pub loader: String,
    pub game: String,
    pub description: Option<String>,
}

#[async_trait]
impl ModLoaderVersion for ForgeModLoaderVersion {
    async fn install(&self) -> Result<()> {
        todo!()
    }

    async fn uninstall(&self) -> Result<()> {
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
impl ModLoader for ForgeModLoader {
    type T = ForgeModLoaderVersion;

    async fn versions(&self) -> Result<HashMap<String, Vec<ForgeModLoaderVersion>>> {
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
                data.entry(game_version.to_owned())
                    .or_insert(Vec::new())
                    .push(ForgeModLoaderVersion {
                        loader: loader_version.to_owned(),
                        game: game_version.to_owned(),
                        description: None,
                    });
            }
        }
        Ok(data)
    }

    async fn installed(&self, version: VersionStorage) -> Result<Option<ForgeModLoaderVersion>> {
        todo!()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename = "metadata")]
pub(crate) struct MavenMetadataBody {
    pub versioning: MavenMetadataVersioning,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MavenMetadataVersioning {
    pub versions: MavenMetadataVersion,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MavenMetadataVersion {
    pub version: Vec<String>,
}
