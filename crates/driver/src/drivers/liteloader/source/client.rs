use elemental_schema::fabric::ProfileJson;

use crate::{
    error::Result,
    http::{HttpSource, fetch_json},
};

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
        build_profile_json(self.inner.client()?, self.endpoints(), &selected).await
    }

    async fn manifest(&self) -> Result<LiteLoaderManifest> {
        let url = self.endpoints().versions_manifest_url()?;
        fetch_json(self.inner.client()?, url.as_str(), "liteloader source").await
    }
}
