use anyhow::{Context, Result};
use elemental_schema::mojang::{
    launcher::LaunchMetaData,
    piston::{PistonMetaAssetIndexObjects, PistonMetaData},
};

use crate::{
    drivers::vanilla::prepared::ResolvedVanillaMetadata,
    families::version_json::VersionJsonRemoteResolver,
    http::{build_default_client, fetch_json},
    url::{Origin, OriginPolicy},
};

const LAUNCHERMETA_ORIGIN: &str = "https://launchermeta.mojang.com";
const PISTONMETA_ORIGIN: &str = "https://piston-meta.mojang.com";
const PISTONDATA_ORIGIN: &str = "https://piston-data.mojang.com";
const RESOURCES_ORIGIN: &str = "https://resources.download.minecraft.net";
const LIBRARIES_ORIGIN: &str = "https://libraries.minecraft.net";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VanillaOrigin {
    LauncherMeta,
    PistonMeta,
    PistonData,
    Resources,
    Libraries,
}

#[derive(Debug, Clone)]
pub struct VanillaEndpoints {
    origin_policy: OriginPolicy<VanillaOrigin>,
}

#[derive(Debug, Clone)]
pub struct VanillaSource {
    client: reqwest::Client,
    endpoints: VanillaEndpoints,
}

impl Default for VanillaEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl Origin for VanillaOrigin {
    fn canonical(self) -> &'static str {
        match self {
            VanillaOrigin::LauncherMeta => LAUNCHERMETA_ORIGIN,
            VanillaOrigin::PistonMeta => PISTONMETA_ORIGIN,
            VanillaOrigin::PistonData => PISTONDATA_ORIGIN,
            VanillaOrigin::Resources => RESOURCES_ORIGIN,
            VanillaOrigin::Libraries => LIBRARIES_ORIGIN,
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[VanillaOrigin] = &[
            VanillaOrigin::LauncherMeta,
            VanillaOrigin::PistonMeta,
            VanillaOrigin::PistonData,
            VanillaOrigin::Resources,
            VanillaOrigin::Libraries,
        ];
        ALL
    }
}

impl VanillaEndpoints {
    pub fn new(origin_policy: OriginPolicy<VanillaOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn mirror(
        launchermeta_origin: String,
        pistonmeta_origin: String,
        pistondata_origin: String,
        resources_origin: String,
        libraries_origin: String,
    ) -> Result<Self> {
        let policy = OriginPolicy::default()
            .with_override(VanillaOrigin::LauncherMeta, launchermeta_origin)?
            .with_override(VanillaOrigin::PistonMeta, pistonmeta_origin)?
            .with_override(VanillaOrigin::PistonData, pistondata_origin)?
            .with_override(VanillaOrigin::Resources, resources_origin)?
            .with_override(VanillaOrigin::Libraries, libraries_origin)?;
        Ok(Self::new(policy))
    }

    pub fn origin_policy(&self) -> &OriginPolicy<VanillaOrigin> {
        &self.origin_policy
    }

    pub fn version_manifest_url(&self) -> Result<String> {
        self.origin_policy.resolve(
            VanillaOrigin::LauncherMeta,
            "/mc/game/version_manifest_v2.json",
        )
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.origin_policy.rewrite_origin_url(raw_url)
    }

    pub fn object_url(&self, hash: impl AsRef<str>) -> Result<String> {
        let hash = hash.as_ref();
        let prefix = hash.get(0..2).context("asset hash is too short")?;
        self.origin_policy
            .resolve(VanillaOrigin::Resources, &format!("{prefix}/{hash}"))
    }
}

impl VersionJsonRemoteResolver for VanillaEndpoints {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.rewrite_upstream(raw_url)
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.object_url(hash)
    }
}

impl Default for VanillaSource {
    fn default() -> Self {
        Self {
            client: build_default_client("vanilla source"),
            endpoints: VanillaEndpoints::default(),
        }
    }
}

impl VanillaSource {
    pub fn new(endpoints: VanillaEndpoints) -> Self {
        Self {
            endpoints,
            ..Self::default()
        }
    }

    pub fn with_client(endpoints: VanillaEndpoints, client: reqwest::Client) -> Self {
        Self { client, endpoints }
    }

    pub fn endpoints(&self) -> &VanillaEndpoints {
        &self.endpoints
    }

    pub async fn launch_meta(&self) -> Result<LaunchMetaData> {
        let url = self.endpoints.version_manifest_url()?;
        fetch_json(&self.client, url.as_str(), "vanilla source").await
    }

    pub async fn piston_meta(&self, url: impl AsRef<str>) -> Result<PistonMetaData> {
        let url = self.endpoints.rewrite_upstream(url.as_ref())?;
        fetch_json(&self.client, url.as_str(), "vanilla source").await
    }

    pub async fn asset_index_objects(
        &self,
        url: impl AsRef<str>,
    ) -> Result<PistonMetaAssetIndexObjects> {
        let url = self.endpoints.rewrite_upstream(url.as_ref())?;
        fetch_json(&self.client, url.as_str(), "vanilla source").await
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
        .with_context(|| format!("can't find vanilla version named '{game_version}'"))?
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

    rewrite_family()
        .with_context(|| format!("rewrite {family_name} upstream url failed for '{raw_url}'"))
}
