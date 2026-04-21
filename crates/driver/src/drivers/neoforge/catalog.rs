use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::catalog::{
    Catalog, GameVersions, Release, ReleaseInfo, collect_single_game_releases,
    single_game_release_info,
};

use super::source::NeoForgeSource;

const GAME_VERSION_HEURISTIC_DESCRIPTION: &str = "Game version is inferred from NeoForge version naming. Installer profile remains the source of truth.";

#[derive(Debug, Clone)]
pub struct NeoForgeCatalog {
    source: NeoForgeSource,
}

#[derive(Debug, Clone)]
pub struct NeoForgeRelease {
    pub loader: String,
    pub game_version_hint: String,
    pub description: Option<String>,
}

impl Default for NeoForgeCatalog {
    fn default() -> Self {
        Self::new(NeoForgeSource::default())
    }
}

impl NeoForgeCatalog {
    pub fn new(source: NeoForgeSource) -> Self {
        Self { source }
    }

    pub fn source(&self) -> &NeoForgeSource {
        &self.source
    }
}

#[async_trait]
impl Release for NeoForgeRelease {
    async fn info(&self) -> ReleaseInfo {
        single_game_release_info(
            self.loader.clone(),
            self.game_version_hint.clone(),
            self.description.clone(),
        )
    }
}

#[async_trait]
impl Catalog for NeoForgeCatalog {
    type Release = NeoForgeRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>> {
        let metadata = self.source.maven_metadata().await?;
        Ok(collect_single_game_releases(
            metadata.versioning.versions.version,
            |version| {
                let game_version_hint = infer_game_version_from_loader_version(version.as_str())?;
                Some((
                    game_version_hint.clone(),
                    NeoForgeRelease {
                        loader: version,
                        game_version_hint,
                        description: Some(GAME_VERSION_HEURISTIC_DESCRIPTION.to_owned()),
                    },
                ))
            },
        ))
    }
}

fn infer_game_version_from_loader_version(loader_version: &str) -> Option<String> {
    let core_version = loader_version.split('-').next().unwrap_or(loader_version);
    let parts = core_version.split('.').collect::<Vec<&str>>();
    let [major, minor, ..] = parts.as_slice() else {
        return None;
    };

    Some(format!("1.{major}.{minor}"))
}
