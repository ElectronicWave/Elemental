use std::collections::HashSet;

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    error::Result,
    families::installer::{InstallerMavenSource, MavenInstallerSpec},
};

use super::{GameVersion, GameVersionCatalog, LoaderRelease, LoaderVersionCatalog};

pub struct InstallerMavenCatalog {
    source: InstallerMavenSource,
}

impl InstallerMavenCatalog {
    pub fn new(source: InstallerMavenSource) -> Self {
        Self { source }
    }

    pub fn for_spec(spec: &'static MavenInstallerSpec) -> Self {
        Self::new(InstallerMavenSource::official(spec))
    }

    pub fn source(&self) -> &InstallerMavenSource {
        &self.source
    }
}

#[async_trait]
impl GameVersionCatalog for InstallerMavenCatalog {
    async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        let spec = self.source.spec();
        if let Some(fixed_game_version) = spec.fixed_game_version {
            return Ok(vec![GameVersion {
                id: MinecraftVersionId::from(fixed_game_version),
                channel: None,
            }]);
        }

        let metadata = self.source.maven_metadata().await?;
        let mut seen = HashSet::new();
        let mut game_versions = Vec::new();
        for version in metadata.versioning.versions.version {
            let Some((game_version, release)) = spec.parse_release(version) else {
                continue;
            };
            if seen.insert(game_version.clone()) {
                game_versions.push(GameVersion {
                    id: game_version,
                    channel: release.channel,
                });
            }
        }
        Ok(game_versions)
    }
}

#[async_trait]
impl LoaderVersionCatalog for InstallerMavenCatalog {
    async fn loader_versions(
        &self,
        game_version: &MinecraftVersionId,
    ) -> Result<Vec<LoaderRelease>> {
        let spec = self.source.spec();
        let metadata = self.source.maven_metadata().await?;
        Ok(metadata
            .versioning
            .versions
            .version
            .into_iter()
            .filter_map(|version| spec.parse_release(version))
            .filter(|(_, release)| &release.game_version == game_version)
            .map(|(_, release)| release)
            .collect())
    }
}
