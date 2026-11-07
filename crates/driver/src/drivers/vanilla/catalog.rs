use async_trait::async_trait;

use crate::{
    catalog::{GameVersion, GameVersionCatalog},
    error::Result,
};

use super::source::VanillaSource;

pub struct VanillaCatalog {
    source: VanillaSource,
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
impl GameVersionCatalog for VanillaCatalog {
    async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        let manifest = self.source.launch_meta().await?;
        Ok(manifest
            .versions
            .into_iter()
            .map(|version| GameVersion {
                id: version.id.into(),
                description: None,
                stream: Some(version.release_type),
            })
            .collect())
    }
}
