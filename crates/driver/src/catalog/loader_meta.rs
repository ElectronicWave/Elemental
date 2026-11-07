use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    error::Result,
    families::version_json::{LoaderMetaEndpoints, LoaderMetaSource},
};

use super::{GameVersion, GameVersionCatalog, LoaderRelease, LoaderVersionCatalog};

pub struct LoaderMetaCatalog<E>
where
    E: LoaderMetaEndpoints,
{
    source: LoaderMetaSource<E>,
}

impl<E> LoaderMetaCatalog<E>
where
    E: LoaderMetaEndpoints,
{
    pub fn new(source: LoaderMetaSource<E>) -> Self {
        Self { source }
    }

    pub fn with_defaults() -> Self
    where
        E: Default,
    {
        Self::new(LoaderMetaSource::default())
    }

    pub fn source(&self) -> &LoaderMetaSource<E> {
        &self.source
    }
}

#[async_trait]
impl<E> GameVersionCatalog for LoaderMetaCatalog<E>
where
    E: LoaderMetaEndpoints,
{
    async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        Ok(self
            .source
            .game_versions()
            .await?
            .into_iter()
            .map(E::game_version)
            .collect())
    }
}

#[async_trait]
impl<E> LoaderVersionCatalog for LoaderMetaCatalog<E>
where
    E: LoaderMetaEndpoints,
{
    async fn loader_versions(
        &self,
        game_version: &MinecraftVersionId,
    ) -> Result<Vec<LoaderRelease>> {
        Ok(self
            .source
            .loader_versions(game_version.as_str())
            .await?
            .into_iter()
            .map(|loader| E::release(game_version.clone(), loader))
            .collect())
    }
}
