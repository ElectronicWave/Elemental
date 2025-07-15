use async_trait::async_trait;
use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
};

#[async_trait]
pub trait ModLoader {
    type ModVersion: ModLoaderVersion;
    async fn versions(&self) -> Result<HashMap<String, Self::ModVersion>>;
    async fn versions_slim(&self) -> Result<HashMap<String, Self::ModVersion>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "`versions_slim` not implemented",
        ))
    }

    async fn installed(&self) -> Result<Option<impl ModLoaderVersion>>;
}

#[async_trait]
pub trait ModLoaderVersion {
    async fn install(&self) -> Result<()>;
    async fn uninstall(&self) -> Result<()>;
    async fn info(&self) -> Result<ModLoaderVersionInfo>;
}

pub struct ModLoaderVersionInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}
