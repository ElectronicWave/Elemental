use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::catalog::{
    Catalog, Release, ReleaseInfo, push_single_game_release, single_game_release_info,
};
use crate::loader_version::LoaderVersionId;

use super::source::RiftSource;

pub struct RiftCatalog {
    source: RiftSource,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RiftCatalogRelease {
    pub game_version: MinecraftVersionId,
    pub loader_version: LoaderVersionId,
    pub published_at: Option<String>,
}

#[async_trait]
impl Release for RiftCatalogRelease {
    async fn info(&self) -> ReleaseInfo {
        single_game_release_info(
            self.loader_version.to_string(),
            self.game_version.clone(),
            self.published_at.clone(),
        )
    }
}

impl RiftCatalog {
    pub fn new(source: RiftSource) -> Self {
        Self { source }
    }

    pub fn with_defaults() -> Self {
        Self::new(RiftSource::default())
    }
}

#[async_trait]
impl Catalog for RiftCatalog {
    type Release = RiftCatalogRelease;

    async fn releases(&self) -> Result<HashMap<MinecraftVersionId, Vec<Self::Release>>> {
        let mut releases = HashMap::new();

        for release in self.source.releases().await? {
            let profile = self.source.profile_json_for_release(&release).await?;
            let game_version = MinecraftVersionId::from(profile.inherits_from);
            push_single_game_release(
                &mut releases,
                game_version.clone(),
                RiftCatalogRelease {
                    game_version,
                    loader_version: LoaderVersionId::from(release.loader_version),
                    published_at: release.published_at,
                },
            );
        }

        Ok(releases)
    }
}
