use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use tokio::sync::{Mutex, OnceCell};

use crate::{
    error::{Error, Result},
    families::version_json::{LoaderMetaEndpoints, LoaderMetaSource},
};

use super::{GameVersion, GameVersionCatalog, LoaderRelease, LoaderVersionCatalog};

pub struct LoaderMetaCatalog<E>
where
    E: LoaderMetaEndpoints,
{
    source: LoaderMetaSource<E>,
    game_versions: OnceCell<Vec<GameVersion>>,
    loader_versions: Mutex<HashMap<MinecraftVersionId, Arc<OnceCell<Vec<LoaderRelease>>>>>,
}

impl<E> LoaderMetaCatalog<E>
where
    E: LoaderMetaEndpoints,
{
    pub fn new(source: LoaderMetaSource<E>) -> Self {
        Self {
            source,
            game_versions: OnceCell::new(),
            loader_versions: Mutex::new(HashMap::new()),
        }
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
            .game_versions
            .get_or_try_init(|| async {
                Ok::<Vec<GameVersion>, Error>(
                    self.source
                        .game_versions()
                        .await?
                        .into_iter()
                        .map(E::game_version)
                        .collect(),
                )
            })
            .await?
            .clone())
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
        let cell = self
            .loader_versions
            .lock()
            .await
            .entry(game_version.clone())
            .or_default()
            .clone();

        cell.get_or_try_init(|| async {
            Ok::<Vec<LoaderRelease>, Error>(
                self.source
                    .loader_versions(game_version.as_str())
                    .await?
                    .into_iter()
                    .map(|loader| E::release(game_version.clone(), loader))
                    .collect(),
            )
        })
        .await
        .cloned()
    }
}
