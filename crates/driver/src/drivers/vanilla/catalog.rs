use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::catalog::{Catalog, GameVersions, Release, ReleaseInfo};

use super::source::VanillaSource;

pub struct VanillaCatalog {
    source: VanillaSource,
}

pub struct VanillaRelease {
    pub version_id: String,
    pub description: Option<String>,
}

#[async_trait]
impl Release for VanillaRelease {
    async fn info(&self) -> ReleaseInfo {
        ReleaseInfo {
            name: self.version_id.clone(),
            game_versions: GameVersions::Single(self.version_id.clone()),
            description: self.description.clone(),
        }
    }
}

impl VanillaCatalog {
    pub fn new(source: VanillaSource) -> Self {
        Self { source }
    }

    pub fn with_defaults() -> Self {
        Self::new(VanillaSource::default())
    }
}

#[async_trait]
impl Catalog for VanillaCatalog {
    type Release = VanillaRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>> {
        let mut releases = HashMap::new();
        let manifest = self.source.launch_meta().await?;

        for version in manifest.versions {
            releases
                .entry(GameVersions::Single(version.id.clone()))
                .or_insert(Vec::new())
                .push(VanillaRelease {
                    version_id: version.id,
                    description: Some(version.release_type),
                });
        }

        Ok(releases)
    }
}
