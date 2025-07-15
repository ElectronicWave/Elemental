// https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml
// https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json

use crate::base::{ModLoader, ModLoaderVersion};
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::Result;

pub struct ForgeModLoader {}
pub struct ForgeModLoaderVersion {}

#[async_trait]
impl ModLoaderVersion for ForgeModLoaderVersion {
    async fn install(&self) -> Result<()> {
        todo!()
    }

    async fn uninstall(&self) -> Result<()> {
        todo!()
    }

    async fn info(&self) -> Result<crate::base::ModLoaderVersionInfo> {
        todo!()
    }
}

#[async_trait]
impl ModLoader for ForgeModLoader {
    type ModVersion = ForgeModLoaderVersion;

    async fn versions(&self) -> Result<HashMap<String, ForgeModLoaderVersion>> {
        todo!()
    }

    async fn installed(&self) -> Result<Option<ForgeModLoaderVersion>> {
        todo!()
    }
}
