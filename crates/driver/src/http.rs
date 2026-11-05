use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct HttpSource<E> {
    client: reqwest::Client,
    endpoints: E,
}

impl<E> HttpSource<E> {
    pub fn new(endpoints: E, source_name: &str) -> Self {
        Self::with_client(endpoints, build_default_client(source_name))
    }

    pub fn with_client(endpoints: E, client: reqwest::Client) -> Self {
        Self { client, endpoints }
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub fn endpoints(&self) -> &E {
        &self.endpoints
    }
}

pub fn build_default_client(source_name: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(format!("Elemental/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap_or_else(|error| panic!("build {source_name} client failed: {error}"))
}

pub async fn fetch_json<T>(client: &reqwest::Client, url: &str, source_name: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    send_ok(client, url, source_name)
        .await?
        .json::<T>()
        .await
        .map_err(|source| Error::DecodeResource {
            source_name: source_name.to_owned(),
            url: url.to_owned(),
            source,
        })
}

pub async fn fetch_text(client: &reqwest::Client, url: &str, source_name: &str) -> Result<String> {
    send_ok(client, url, source_name)
        .await?
        .text()
        .await
        .map_err(|source| Error::DecodeResource {
            source_name: source_name.to_owned(),
            url: url.to_owned(),
            source,
        })
}

pub async fn fetch_bytes(
    client: &reqwest::Client,
    url: &str,
    source_name: &str,
) -> Result<Vec<u8>> {
    send_ok(client, url, source_name)
        .await?
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|source| Error::DecodeResource {
            source_name: source_name.to_owned(),
            url: url.to_owned(),
            source,
        })
}

async fn send_ok(
    client: &reqwest::Client,
    url: &str,
    source_name: &str,
) -> Result<reqwest::Response> {
    client
        .get(url)
        .send()
        .await
        .map_err(|source| Error::RequestResource {
            source_name: source_name.to_owned(),
            url: url.to_owned(),
            source,
        })?
        .error_for_status()
        .map_err(|source| Error::HttpStatus {
            source_name: source_name.to_owned(),
            url: url.to_owned(),
            source,
        })
}
