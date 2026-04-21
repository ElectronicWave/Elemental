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

pub fn single_game_release_info(
    name: String,
    game_version: String,
    description: Option<String>,
) -> ReleaseInfo {
    ReleaseInfo {
        name,
        game_versions: GameVersions::Single(game_version),
        description,
    }
}

pub fn push_single_game_release<R>(
    releases: &mut HashMap<GameVersions, Vec<R>>,
    game_version: String,
    release: R,
) {
    releases
        .entry(GameVersions::Single(game_version))
        .or_default()
        .push(release);
}
