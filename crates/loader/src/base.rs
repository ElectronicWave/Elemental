use anyhow::{Result, bail};
use async_trait::async_trait;
use elemental_core::storage::{layout::Layout, version::VersionStorage};
use std::collections::HashMap;
#[async_trait]
pub trait ModLoader {
    type T: ModLoaderVersion;
    /// Map { GameVersion: Vec<LoaderVersion>}
    async fn versions(&self) -> Result<HashMap<Version, Vec<Self::T>>>;
    async fn versions_slim(&self) -> Result<HashMap<Version, Vec<Self::T>>> {
        bail!("`versions_slim` not implemented");
    }

    async fn installed<L: Layout, VL: Layout>(
        &self,
        version: VersionStorage<L, VL>,
    ) -> Result<Option<impl ModLoaderVersion>>;
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
    pub version: Version,
    /// e.g. `Beta`/`Recommand`/`Latest`/...
    pub description: Option<String>,
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum Version {
    SINGLE(String),
    MULTI(Vec<String>),
    IGNORE,
}
