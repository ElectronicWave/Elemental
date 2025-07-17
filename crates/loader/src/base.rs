use async_trait::async_trait;
use elemental_core::storage::version::VersionStorage;
use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
};

#[async_trait]
pub trait ModLoader {
    type T: ModLoaderVersion;
    /// Map { GameVersion: Vec<LoaderVersion>}
    async fn versions(&self) -> Result<HashMap<String, Vec<Self::T>>>;
    async fn versions_slim(&self) -> Result<HashMap<String, Vec<Self::T>>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "`versions_slim` not implemented",
        ))
    }

    async fn installed(&self, version: VersionStorage) -> Result<Option<impl ModLoaderVersion>>;
}

#[async_trait]
pub trait ModLoaderVersion {
    async fn install(&self) -> Result<()>;
    async fn uninstall(&self) -> Result<()>;
    async fn info(&self) -> ModLoaderVersionInfo;
}

pub struct ModLoaderVersionInfo {
    /// Usually ModLoader Version Name
    pub name: String,
    /// Game Version
    pub version: String,
    /// e.g. `Beta`/`Recommand`/`Latest`/...
    pub description: Option<String>,
}
