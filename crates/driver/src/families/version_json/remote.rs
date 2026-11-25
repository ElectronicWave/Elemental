use std::fmt::Debug;

use elemental_schema::mojang::{
    launcher::LaunchMetaData,
    piston::{PistonMetaAssetIndexObjects, PistonMetaData},
};

use crate::{
    error::{Error, Result},
    http::HttpClient,
    url::{Origin, OriginPolicy},
};

const LAUNCHERMETA_ORIGIN: &str = "https://launchermeta.mojang.com";
const PISTONMETA_ORIGIN: &str = "https://piston-meta.mojang.com";
const PISTONDATA_ORIGIN: &str = "https://piston-data.mojang.com";
const RESOURCES_ORIGIN: &str = "https://resources.download.minecraft.net";
const LIBRARIES_ORIGIN: &str = "https://libraries.minecraft.net";

const KNOWN_VANILLA_ORIGINS: &[&str] = &[
    LAUNCHERMETA_ORIGIN,
    PISTONMETA_ORIGIN,
    PISTONDATA_ORIGIN,
    RESOURCES_ORIGIN,
    LIBRARIES_ORIGIN,
];

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
    origin_policy: OriginPolicy,
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

impl AsRef<str> for VanillaOrigin {
    fn as_ref(&self) -> &str {
        self.canonical()
    }
}

impl VanillaEndpoints {
    pub fn new(origin_policy: OriginPolicy) -> Self {
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

    pub fn origin_policy(&self) -> &OriginPolicy {
        &self.origin_policy
    }

    pub fn version_manifest_url(&self) -> Result<String> {
        self.origin_policy.resolve(
            VanillaOrigin::LauncherMeta,
            "/mc/game/version_manifest_v2.json",
        )
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.origin_policy
            .rewrite_origin_url(KNOWN_VANILLA_ORIGINS, raw_url)
    }

    pub fn object_url(&self, hash: impl AsRef<str>) -> Result<String> {
        let hash = hash.as_ref();
        let prefix = hash.get(0..2).ok_or_else(|| Error::AssetHashTooShort {
            hash: hash.to_owned(),
        })?;
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

#[derive(Debug, Clone)]
pub struct VanillaSource {
    client: HttpClient,
    endpoints: VanillaEndpoints,
}

impl Default for VanillaSource {
    fn default() -> Self {
        Self::new(VanillaEndpoints::default())
    }
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
            client: HttpClient::new("vanilla source"),
            endpoints,
        }
    }

    pub fn with_client(endpoints: VanillaEndpoints, client: reqwest::Client) -> Self {
        Self {
            client: HttpClient::with_client(client),
            endpoints,
        }
    }

    pub fn endpoints(&self) -> &VanillaEndpoints {
        &self.endpoints
    }

    pub async fn launch_meta(&self) -> Result<LaunchMetaData> {
        let url = self.endpoints.version_manifest_url()?;
        self.client.fetch_json(url.as_str(), "vanilla source").await
    }

    pub async fn piston_meta(&self, url: impl AsRef<str>) -> Result<PistonMetaData> {
        let url = self.endpoints.rewrite_upstream(url.as_ref())?;
        self.client.fetch_json(url.as_str(), "vanilla source").await
    }

    pub async fn asset_index_objects(
        &self,
        url: impl AsRef<str>,
    ) -> Result<PistonMetaAssetIndexObjects> {
        let url = self.endpoints.rewrite_upstream(url.as_ref())?;
        self.client.fetch_json(url.as_str(), "vanilla source").await
    }
}

pub trait VersionJsonRemoteResolver: Debug + Send + Sync + 'static {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String>;
    fn object_url(&self, hash: &str) -> Result<String>;

    fn artifact_url(&self, raw_url: &str, _artifact_path: &str) -> Result<String> {
        self.rewrite_upstream(raw_url)
    }
}

pub trait UpstreamUrlRewriter: Clone + Debug + Send + Sync + 'static {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String>;
}

#[derive(Debug, Clone)]
pub struct VanillaFallbackRemoteResolver<E>
where
    E: UpstreamUrlRewriter,
{
    family_name: &'static str,
    vanilla_endpoints: VanillaEndpoints,
    family_endpoints: E,
}

impl<E> VanillaFallbackRemoteResolver<E>
where
    E: UpstreamUrlRewriter,
{
    pub fn new(
        family_name: &'static str,
        vanilla_endpoints: VanillaEndpoints,
        family_endpoints: E,
    ) -> Self {
        Self {
            family_name,
            vanilla_endpoints,
            family_endpoints,
        }
    }
}

impl<E> VersionJsonRemoteResolver for VanillaFallbackRemoteResolver<E>
where
    E: UpstreamUrlRewriter,
{
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        rewrite_upstream_with_vanilla_fallback(
            &self.vanilla_endpoints,
            raw_url,
            self.family_name,
            || self.family_endpoints.rewrite_upstream(raw_url),
        )
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }
}
