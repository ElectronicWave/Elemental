#![cfg_attr(not(feature = "driver-all"), allow(dead_code))]

use elemental_core::http::SwappableClient;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct HttpSource<E> {
    client: std::result::Result<SwappableClient, String>,
    headers: HeaderMap,
    endpoints: E,
}

impl<E> HttpSource<E> {
    pub fn new(endpoints: E, source_name: &str) -> Self {
        Self {
            client: build_default_swappable_client(source_name),
            headers: HeaderMap::new(),
            endpoints,
        }
    }

    pub fn with_client(endpoints: E, client: reqwest::Client) -> Self {
        Self::with_http_client(endpoints, SwappableClient::from_client(client))
    }

    pub fn with_http_client(endpoints: E, client: SwappableClient) -> Self {
        Self {
            client: Ok(client),
            headers: HeaderMap::new(),
            endpoints,
        }
    }

    /// Replaces per-source headers sent with requests made by this source.
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    /// Inserts a per-source header (such as a mirror-specific `X-KEY`).
    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    pub fn client(&self) -> Result<reqwest::Client> {
        self.client
            .as_ref()
            .map(SwappableClient::client)
            .map_err(|message| Error::HttpClientBuild {
                message: message.clone(),
            })
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn endpoints(&self) -> &E {
        &self.endpoints
    }

    pub async fn fetch_json<T>(&self, url: &str, source_name: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let client = self.client()?;
        fetch_json_with_headers(&client, url, source_name, &self.headers).await
    }

    pub async fn fetch_bytes(&self, url: &str, source_name: &str) -> Result<Vec<u8>> {
        let client = self.client()?;
        fetch_bytes_with_headers(&client, url, source_name, &self.headers).await
    }
}

fn build_default_swappable_client(
    source_name: &str,
) -> std::result::Result<SwappableClient, String> {
    SwappableClient::try_default()
        .map_err(|error| format!("build {source_name} client failed: {error}"))
}

pub async fn fetch_text(client: &reqwest::Client, url: &str, source_name: &str) -> Result<String> {
    fetch_text_with_headers(client, url, source_name, &HeaderMap::new()).await
}

pub async fn fetch_text_with_headers(
    client: &reqwest::Client,
    url: &str,
    source_name: &str,
    headers: &HeaderMap,
) -> Result<String> {
    let bytes = fetch_bytes_with_headers(client, url, source_name, headers).await?;
    String::from_utf8(bytes).map_err(|source| Error::DecodeResource {
        source_name: source_name.to_owned(),
        url: url.to_owned(),
        source: Box::new(source),
    })
}

async fn fetch_json_with_headers<T>(
    client: &reqwest::Client,
    url: &str,
    source_name: &str,
    headers: &HeaderMap,
) -> Result<T>
where
    T: DeserializeOwned,
{
    let bytes = fetch_bytes_with_headers(client, url, source_name, headers).await?;
    serde_json::from_slice(&bytes).map_err(|source| Error::DecodeResource {
        source_name: source_name.to_owned(),
        url: url.to_owned(),
        source: Box::new(source),
    })
}

async fn fetch_bytes_with_headers(
    client: &reqwest::Client,
    url: &str,
    source_name: &str,
    headers: &HeaderMap,
) -> Result<Vec<u8>> {
    send_ok(client, url, source_name, headers)
        .await?
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|source| Error::DecodeResource {
            source_name: source_name.to_owned(),
            url: url.to_owned(),
            source: Box::new(source),
        })
}

async fn send_ok(
    client: &reqwest::Client,
    url: &str,
    source_name: &str,
    headers: &HeaderMap,
) -> Result<reqwest::Response> {
    let mut request = client.get(url);
    if !headers.is_empty() {
        request = request.headers(headers.clone());
    }
    request
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
