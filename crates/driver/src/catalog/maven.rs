use std::collections::HashSet;

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    error::Result,
    families::installer::{InstallerMavenEndpoints, InstallerMavenSource},
};

use super::{GameVersion, GameVersionCatalog, LoaderRelease, LoaderVersionCatalog};

pub struct InstallerMavenCatalog<E>
where
    E: InstallerMavenEndpoints,
{
    source: InstallerMavenSource<E>,
}

impl<E> InstallerMavenCatalog<E>
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
impl<E> GameVersionCatalog for InstallerMavenCatalog<E>
where
    E: InstallerMavenEndpoints,
{
    async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        let metadata = self.source.maven_metadata().await?;
        let mut seen = HashSet::new();
        let mut game_versions = Vec::new();
        for version in metadata.versioning.versions.version {
            let Some((game_version, release)) = E::release(version) else {
                continue;
            };
            if seen.insert(game_version.clone()) {
                game_versions.push(GameVersion {
                    id: game_version,
                    description: release.description,
                    stream: release.stream,
                });
            }
        }
        Ok(game_versions)
    }
}

#[async_trait]
impl<E> LoaderVersionCatalog for InstallerMavenCatalog<E>
where
    E: InstallerMavenEndpoints,
{
    async fn loader_versions(
        &self,
        game_version: &MinecraftVersionId,
    ) -> Result<Vec<LoaderRelease>> {
        let metadata = self.source.maven_metadata().await?;
        Ok(metadata
            .versioning
            .versions
            .version
            .into_iter()
            .filter_map(E::release)
            .filter(|(_, release)| &release.game_version == game_version)
            .map(|(_, release)| release)
            .collect())
    }
}
