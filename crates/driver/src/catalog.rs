use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Catalog {
    type Release: Release;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>>;
}

#[async_trait]
pub trait Release {
    async fn install(&self) -> Result<()>;
    async fn uninstall(&self) -> Result<()>;
    async fn info(&self) -> ReleaseInfo;
}

pub struct ReleaseInfo {
    pub name: String,
    pub game_versions: GameVersions,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum GameVersions {
    Single(String),
    Multi(Vec<String>),
    Ignore,
}
