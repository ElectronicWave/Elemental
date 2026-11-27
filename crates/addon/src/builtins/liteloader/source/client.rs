use elemental_core::http::{RequestBuilderExt, SwappableClient};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::error::Result;

use super::super::manifest::{LiteLoaderManifest, collect_releases};
use super::LiteLoaderRelease;
use super::endpoints::LiteLoaderEndpoints;

#[derive(Debug, Clone)]
pub struct LiteLoaderSource {
    client: SwappableClient,
    endpoints: LiteLoaderEndpoints,
}

impl Default for LiteLoaderSource {
    fn default() -> Self {
        Self::new(LiteLoaderEndpoints::default())
    }
}

impl LiteLoaderSource {
    pub fn new(endpoints: LiteLoaderEndpoints) -> Self {
        Self {
            client: SwappableClient::default(),
            endpoints,
        }
    }

    pub fn with_client(endpoints: LiteLoaderEndpoints, client: reqwest::Client) -> Self {
        Self {
            client: SwappableClient::from_client(client),
            endpoints,
        }
    }

    pub fn with_http_client(mut self, client: SwappableClient) -> Self {
        self.client = client;
        self
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.client = self.client.with_headers(headers);
        self
    }

    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.client = self.client.with_header(name, value);
        self
    }

    pub fn endpoints(&self) -> &LiteLoaderEndpoints {
        &self.endpoints
    }

    pub(in super::super) fn http_client(&self) -> &SwappableClient {
        &self.client
    }

    pub async fn releases(&self) -> Result<Vec<LiteLoaderRelease>> {
        let manifest = self.manifest().await?;
        Ok(collect_releases(manifest))
    }

    pub(in super::super) async fn manifest(&self) -> Result<LiteLoaderManifest> {
        let url = self.endpoints.versions_manifest_url()?;
        Ok(self.client.get(url.as_str()).send_json().await?)
    }
}
