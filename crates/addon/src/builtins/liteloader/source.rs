mod endpoints;
mod rewrite;

use elemental_core::http::{RequestBuilderExt, SwappableClient};

use crate::error::Result;

use super::manifest::{LiteLoaderManifest, collect_releases};

pub use super::manifest::LiteLoaderRelease;
pub use endpoints::{LiteLoaderEndpoints, LiteLoaderOrigin};

/// Remote metadata client for fetching LiteLoader version manifests.
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
    /// Creates a `LiteLoaderSource` targeting `endpoints` with a default HTTP client.
    pub fn new(endpoints: LiteLoaderEndpoints) -> Self {
        Self {
            client: SwappableClient::default(),
            endpoints,
        }
    }

    /// Replaces the [`SwappableClient`] used for LiteLoader HTTP requests.
    pub fn with_http_client(mut self, client: SwappableClient) -> Self {
        self.client = client;
        self
    }

    /// Returns the configured [`LiteLoaderEndpoints`].
    pub fn endpoints(&self) -> &LiteLoaderEndpoints {
        &self.endpoints
    }

    pub(super) fn http_client(&self) -> &SwappableClient {
        &self.client
    }

    /// Fetches and returns all available [`LiteLoaderRelease`] entries.
    pub async fn releases(&self) -> Result<Vec<LiteLoaderRelease>> {
        let manifest = self.manifest().await?;
        Ok(collect_releases(manifest))
    }

    pub(super) async fn manifest(&self) -> Result<LiteLoaderManifest> {
        let url = self.endpoints.versions_manifest_url()?;
        Ok(self.client.get(url.as_str()).send_json().await?)
    }
}
