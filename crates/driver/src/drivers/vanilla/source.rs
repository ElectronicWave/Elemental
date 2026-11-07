use elemental_schema::mojang::{
    launcher::LaunchMetaData,
    piston::{PistonMetaAssetIndexObjects, PistonMetaData},
};

use crate::{
    error::Result,
    http::{HttpSource, fetch_json},
};

pub use super::endpoints::{VanillaEndpoints, VanillaOrigin};

mod resolve;
mod rewrite;

pub use resolve::resolve_vanilla_metadata;
pub use rewrite::rewrite_upstream_with_vanilla_fallback;

#[derive(Debug, Clone)]
pub struct VanillaSource {
    inner: HttpSource<VanillaEndpoints>,
}

impl Default for VanillaSource {
    fn default() -> Self {
        Self::new(VanillaEndpoints::default())
    }
}

impl VanillaSource {
    pub fn new(endpoints: VanillaEndpoints) -> Self {
        Self {
            inner: HttpSource::new(endpoints, "vanilla source"),
        }
    }

    pub fn with_client(endpoints: VanillaEndpoints, client: reqwest::Client) -> Self {
        Self {
            inner: HttpSource::with_client(endpoints, client),
        }
    }

    pub fn endpoints(&self) -> &VanillaEndpoints {
        self.inner.endpoints()
    }

    pub async fn launch_meta(&self) -> Result<LaunchMetaData> {
        let url = self.endpoints().version_manifest_url()?;
        fetch_json(self.inner.client()?, url.as_str(), "vanilla source").await
    }

    pub async fn piston_meta(&self, url: impl AsRef<str>) -> Result<PistonMetaData> {
        let url = self.endpoints().rewrite_upstream(url.as_ref())?;
        fetch_json(self.inner.client()?, url.as_str(), "vanilla source").await
    }

    pub async fn asset_index_objects(
        &self,
        url: impl AsRef<str>,
    ) -> Result<PistonMetaAssetIndexObjects> {
        let url = self.endpoints().rewrite_upstream(url.as_ref())?;
        fetch_json(self.inner.client()?, url.as_str(), "vanilla source").await
    }
}
