use crate::mojang::{LaunchMetaData, MojangBaseUrl, PistonMetaAssetIndexObjects, PistonMetaData};
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MojangClient {
    client: reqwest::Client,
    pub baseurl: MojangBaseUrl,
}

impl Default for MojangClient {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent(format!("Elemental/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("build mojang client failed"),
            baseurl: MojangBaseUrl::default(),
        }
    }
}

impl MojangClient {
    pub fn new(baseurl: MojangBaseUrl) -> Self {
        Self {
            baseurl,
            ..Self::default()
        }
    }

    pub fn with_client(baseurl: MojangBaseUrl, client: reqwest::Client) -> Self {
        Self { client, baseurl }
    }

    pub async fn launchmeta(&self) -> Result<LaunchMetaData> {
        let url = self.baseurl.version_manifest_url();
        self.fetch_json(url.as_str()).await
    }

    pub async fn pistonmeta(&self, url: impl Into<String>) -> Result<PistonMetaData> {
        let url = self.baseurl.rewrite_pistonmeta_url(url);
        self.fetch_json(url.as_str()).await
    }

    pub async fn pistonmeta_assetindex_objects(
        &self,
        url: impl Into<String>,
    ) -> Result<PistonMetaAssetIndexObjects> {
        let url = self.baseurl.rewrite_pistonmeta_url(url);
        self.fetch_json(url.as_str()).await
    }

    async fn fetch_json<T>(&self, url: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.client
            .get(url)
            .send()
            .await
            .with_context(|| format!("request mojang resource failed: {url}"))?
            .error_for_status()
            .with_context(|| format!("mojang resource returned error status: {url}"))?
            .json::<T>()
            .await
            .with_context(|| format!("decode mojang resource failed: {url}"))
    }
}

#[tokio::test]
async fn test_client() {
    let client = MojangClient::default();
    let launch_meta = client.launchmeta().await.unwrap();
    let launch_meta_version_data = launch_meta.versions.first().unwrap();
    let pistonmeta = client
        .pistonmeta(launch_meta_version_data.url.clone())
        .await
        .unwrap();
    println!("{pistonmeta:?}")
}
