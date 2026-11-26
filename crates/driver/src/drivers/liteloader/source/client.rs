use elemental_core::http::SwappableClient;
use elemental_schema::fabric::ProfileJson;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::{error::Result, http::HttpSource};

use super::super::manifest::{LiteLoaderManifest, collect_releases, select_build};
use super::super::profile::build_profile_json;
use super::LiteLoaderRelease;
use super::endpoints::LiteLoaderEndpoints;

#[derive(Debug, Clone)]
pub struct LiteLoaderSource {
    inner: HttpSource<LiteLoaderEndpoints>,
}

impl Default for LiteLoaderSource {
    fn default() -> Self {
        Self::new(LiteLoaderEndpoints::default())
    }
}

impl LiteLoaderSource {
    pub fn new(endpoints: LiteLoaderEndpoints) -> Self {
        Self {
            inner: HttpSource::new(endpoints, "liteloader source"),
        }
    }

    pub fn with_client(endpoints: LiteLoaderEndpoints, client: reqwest::Client) -> Self {
        Self {
            inner: HttpSource::with_client(endpoints, client),
        }
    }

    pub fn with_http_client(endpoints: LiteLoaderEndpoints, client: SwappableClient) -> Self {
        Self {
            inner: HttpSource::with_http_client(endpoints, client),
        }
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.inner = self.inner.with_headers(headers);
        self
    }

    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.inner = self.inner.with_header(name, value);
        self
    }

    pub fn endpoints(&self) -> &LiteLoaderEndpoints {
        self.inner.endpoints()
    }

    pub async fn releases(&self) -> Result<Vec<LiteLoaderRelease>> {
        let manifest = self.manifest().await?;
        Ok(collect_releases(manifest))
    }

    pub async fn profile_json(
        &self,
        game_version: &str,
        loader_version: &str,
    ) -> Result<ProfileJson> {
        let manifest = self.manifest().await?;
        let selected = select_build(&manifest, game_version, loader_version)?;
        let client = self.inner.client()?;
        build_profile_json(&client, self.endpoints(), &selected).await
    }

    async fn manifest(&self) -> Result<LiteLoaderManifest> {
        let url = self.endpoints().versions_manifest_url()?;
        self.inner
            .fetch_json(url.as_str(), "liteloader source")
            .await
    }
}
