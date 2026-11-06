use std::collections::HashMap;

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    error::Result,
    families::installer::{InstallerMavenEndpoints, InstallerMavenSource},
};

use super::{Catalog, collect_single_game_releases};

pub struct MavenCatalog<E>
where
    E: InstallerMavenEndpoints,
{
    source: InstallerMavenSource<E>,
}

impl<E> MavenCatalog<E>
where
    E: InstallerMavenEndpoints + Default,
{
    pub fn new(source: InstallerMavenSource<E>) -> Self {
        Self { source }
    }

    pub fn with_defaults() -> Self {
        Self::new(InstallerMavenSource::default())
    }

    pub fn source(&self) -> &InstallerMavenSource<E> {
        &self.source
    }
}

#[async_trait]
impl<E> Catalog for MavenCatalog<E>
where
    E: InstallerMavenEndpoints,
{
    type Release = E::Release;

    async fn releases(&self) -> Result<HashMap<MinecraftVersionId, Vec<Self::Release>>> {
        let metadata = self.source.maven_metadata().await?;
        Ok(collect_single_game_releases(
            metadata.versioning.versions.version,
            |version| E::release(version),
        ))
    }
}
