use elemental_schema::mojang::{
    launcher::LaunchMetaData,
    piston::{PistonMetaAssetIndexObjects, PistonMetaData},
};

use crate::{
    drivers::vanilla::prepared::ResolvedVanillaMetadata,
    error::{Error, Result},
    http::{HttpSource, fetch_json},
};

pub use super::endpoints::{VanillaEndpoints, VanillaOrigin};

#[derive(Debug, Clone)]
pub struct VanillaSource {
    inner: HttpSource<VanillaEndpoints>,
}

impl Default for VanillaSource {
    fn default() -> Self {
        Self::new(VanillaEndpoints::default())
    }
}

pub async fn resolve_vanilla_metadata(
    vanilla_source: &VanillaSource,
    game_version: &str,
) -> Result<ResolvedVanillaMetadata> {
    let launchmeta = vanilla_source.launch_meta().await?;
    let metadata_url = launchmeta
        .versions
        .iter()
        .find(|version| version.id == game_version)
        .ok_or_else(|| Error::VanillaVersionNotFound {
            game_version: game_version.to_owned(),
        })?
        .url
        .clone();
    let metadata = vanilla_source.piston_meta(metadata_url).await?;
    let asset_index_objects = vanilla_source
        .asset_index_objects(&metadata.asset_index.url)
        .await?;

    Ok(ResolvedVanillaMetadata::new(
        vanilla_source.endpoints().clone(),
        metadata,
        asset_index_objects,
    ))
}

pub fn rewrite_upstream_with_vanilla_fallback<RewriteFn>(
    vanilla_endpoints: &VanillaEndpoints,
    raw_url: &str,
    family_name: &str,
    rewrite_family: RewriteFn,
) -> Result<String>
where
    RewriteFn: FnOnce() -> Result<String>,
{
    if let Ok(rewritten) = vanilla_endpoints.rewrite_upstream(raw_url) {
        return Ok(rewritten);
    }

    rewrite_family().map_err(|source| Error::RewriteUpstream {
        family_name: family_name.to_owned(),
        raw_url: raw_url.to_owned(),
        source: Box::new(source),
    })
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
