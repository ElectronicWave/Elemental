use std::fmt::Debug;

use crate::{
    catalog::{GameVersion, GameVersionCatalog, LoaderRelease, LoaderVersionCatalog},
    error::Result,
};
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use serde::de::DeserializeOwned;

use crate::http::HttpClient;

pub trait LoaderMetaEndpoints: Clone + Debug + Send + Sync + 'static {
    type GameVersion: DeserializeOwned;
    type LoaderGameVersion: DeserializeOwned + Send;
    type ProfileJson: DeserializeOwned;

    const SOURCE_NAME: &'static str;

    fn game_versions_url(&self) -> Result<String>;
    fn loader_versions_url(&self, game_version: &str) -> Result<String>;
    fn profile_json_url(&self, game_version: &str, loader_version: &str) -> Result<String>;

    fn game_version(version: Self::GameVersion) -> GameVersion;
    fn release(game_version: MinecraftVersionId, version: Self::LoaderGameVersion)
    -> LoaderRelease;
}

pub trait LoaderProfileEndpoints: LoaderMetaEndpoints {
    type LoaderProfile: DeserializeOwned;

    fn loader_profile_url(&self, game_version: &str, loader_version: &str) -> Result<String>;
}

#[derive(Debug, Clone)]
pub struct LoaderMetaSource<E>
where
    E: LoaderMetaEndpoints,
{
    client: HttpClient,
    endpoints: E,
}

impl<E> Default for LoaderMetaSource<E>
where
    E: LoaderMetaEndpoints + Default,
{
    fn default() -> Self {
        Self::new(E::default())
    }
}

impl<E> LoaderMetaSource<E>
where
    E: LoaderMetaEndpoints,
{
    pub fn new(endpoints: E) -> Self {
        Self {
            client: HttpClient::new(E::SOURCE_NAME),
            endpoints,
        }
    }

    pub fn with_client(endpoints: E, client: reqwest::Client) -> Self {
        Self {
            client: HttpClient::with_client(client),
            endpoints,
        }
    }

    pub fn endpoints(&self) -> &E {
        &self.endpoints
    }

    pub async fn game_versions(&self) -> Result<Vec<E::GameVersion>> {
        let url = self.endpoints.game_versions_url()?;
        self.client.fetch_json(url.as_str(), E::SOURCE_NAME).await
    }

    pub async fn loader_versions(&self, game_version: &str) -> Result<Vec<E::LoaderGameVersion>> {
        let url = self.endpoints.loader_versions_url(game_version)?;
        self.client.fetch_json(url.as_str(), E::SOURCE_NAME).await
    }

    pub async fn profile_json(
        &self,
        game_version: &str,
        loader_version: &str,
    ) -> Result<E::ProfileJson> {
        let url = self
            .endpoints
            .profile_json_url(game_version, loader_version)?;
        self.client.fetch_json(url.as_str(), E::SOURCE_NAME).await
    }
}

impl<E> LoaderMetaSource<E>
where
    E: LoaderProfileEndpoints,
{
    pub async fn loader_profile(
        &self,
        game_version: &str,
        loader_version: &str,
    ) -> Result<E::LoaderProfile> {
        let url = self
            .endpoints
            .loader_profile_url(game_version, loader_version)?;
        self.client.fetch_json(url.as_str(), E::SOURCE_NAME).await
    }
}
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
