use std::collections::HashMap;

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    error::Result,
    families::version_json::{LoaderMetaEndpoints, LoaderMetaSource},
};

use super::{Catalog, collect_single_game_loader_releases};

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
impl<E> Catalog for LoaderMetaCatalog<E>
where
    E: LoaderMetaEndpoints,
{
    type Release = E::Release;

    async fn releases(&self) -> Result<HashMap<MinecraftVersionId, Vec<Self::Release>>> {
        let game_versions = self
            .source
            .game_versions()
            .await?
            .into_iter()
            .map(E::game_version_id)
            .collect::<Vec<MinecraftVersionId>>();

        collect_single_game_loader_releases(
            game_versions,
            |game_version| async move { self.source.loader_versions(game_version.as_str()).await },
            |game_version, loader| E::release(game_version.clone(), loader),
        )
        .await
    }
}
