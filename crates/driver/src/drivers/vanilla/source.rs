use std::time::Duration;

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

use crate::{
    drivers::version_json::{LaunchMetaData, PistonMetaAssetIndexObjects, PistonMetaData},
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

impl Default for VanillaSource {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent(format!("Elemental/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("build vanilla source client failed"),
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
        self.fetch_json(url.as_str()).await
    }

    pub async fn piston_meta(&self, url: impl AsRef<str>) -> Result<PistonMetaData> {
        let url = self.endpoints.rewrite_upstream(url.as_ref())?;
        self.fetch_json(url.as_str()).await
    }

    pub async fn asset_index_objects(
        &self,
        url: impl AsRef<str>,
    ) -> Result<PistonMetaAssetIndexObjects> {
        let url = self.endpoints.rewrite_upstream(url.as_ref())?;
        self.fetch_json(url.as_str()).await
    }

    async fn fetch_json<T>(&self, url: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.client
            .get(url)
            .send()
            .await
            .with_context(|| format!("request vanilla source resource failed: {url}"))?
            .error_for_status()
            .with_context(|| format!("vanilla source returned error status: {url}"))?
            .json::<T>()
            .await
            .with_context(|| format!("decode vanilla source resource failed: {url}"))
    }
}
