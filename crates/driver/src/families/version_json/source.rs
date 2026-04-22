use std::fmt::Debug;

use anyhow::Result;
use serde::de::DeserializeOwned;

use crate::http::{HttpSource, fetch_json};

pub trait LoaderMetaEndpoints: Clone + Debug + Send + Sync + 'static {
    type GameVersion: DeserializeOwned;
    type LoaderGameVersion: DeserializeOwned;
    type ProfileJson: DeserializeOwned;

    const SOURCE_NAME: &'static str;

    fn game_versions_url(&self) -> Result<String>;
    fn loader_versions_url(&self, game_version: &str) -> Result<String>;
    fn profile_json_url(&self, game_version: &str, loader_version: &str) -> Result<String>;
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
    inner: HttpSource<E>,
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
            inner: HttpSource::new(endpoints, E::SOURCE_NAME),
        }
    }

    pub fn with_client(endpoints: E, client: reqwest::Client) -> Self {
        Self {
            inner: HttpSource::with_client(endpoints, client),
        }
    }

    pub fn endpoints(&self) -> &E {
        self.inner.endpoints()
    }

    pub async fn game_versions(&self) -> Result<Vec<E::GameVersion>> {
        let url = self.endpoints().game_versions_url()?;
        fetch_json(self.inner.client(), url.as_str(), E::SOURCE_NAME).await
    }

    pub async fn loader_versions(&self, game_version: &str) -> Result<Vec<E::LoaderGameVersion>> {
        let url = self.endpoints().loader_versions_url(game_version)?;
        fetch_json(self.inner.client(), url.as_str(), E::SOURCE_NAME).await
    }

    pub async fn profile_json(
        &self,
        game_version: &str,
        loader_version: &str,
    ) -> Result<E::ProfileJson> {
        let url = self
            .endpoints()
            .profile_json_url(game_version, loader_version)?;
        fetch_json(self.inner.client(), url.as_str(), E::SOURCE_NAME).await
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
            .endpoints()
            .loader_profile_url(game_version, loader_version)?;
        fetch_json(self.inner.client(), url.as_str(), E::SOURCE_NAME).await
    }
}
