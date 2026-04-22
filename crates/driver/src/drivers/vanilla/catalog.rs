use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::catalog::{
    Catalog, GameVersions, Release, ReleaseInfo, push_single_game_release, single_game_release_info,
};

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
        single_game_release_info(
            self.version_id.clone(),
            MinecraftVersionId::from(self.version_id.clone()),
            self.description.clone(),
        )
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
            let version_id = version.id;
            push_single_game_release(
                &mut releases,
                MinecraftVersionId::from(version_id.clone()),
                VanillaRelease {
                    version_id,
                    description: Some(version.release_type),
                },
            );
        }

        Ok(releases)
    }
}
