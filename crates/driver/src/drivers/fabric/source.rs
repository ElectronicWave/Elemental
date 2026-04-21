use std::time::Duration;

use anyhow::{Context, Result};
use elemental_schema::fabric::{GameVersion, LoaderGameVersion, LoaderProfile, ProfileJson};
use serde::de::DeserializeOwned;

use crate::url::{Origin, OriginPolicy};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FabricFlavor {
    #[default]
    Fabric,
    LegacyFabric,
    Babric,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricEndpointOverrides {
    pub meta_origin: String,
    pub maven_origin: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FabricOrigin {
    Meta,
    Maven,
}

#[derive(Debug, Clone)]
pub struct FabricEndpoints {
    origin_policy: OriginPolicy<FabricOrigin>,
}

#[derive(Debug, Clone)]
pub struct FabricSource {
    client: reqwest::Client,
    endpoints: FabricEndpoints,
}

impl Default for FabricEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl Origin for FabricOrigin {
    fn canonical(self) -> &'static str {
        match self {
            FabricOrigin::Meta => "https://meta.fabricmc.net",
            FabricOrigin::Maven => "https://maven.fabricmc.net",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[FabricOrigin] = &[FabricOrigin::Meta, FabricOrigin::Maven];
        ALL
    }
}

impl FabricEndpoints {
    pub fn new(origin_policy: OriginPolicy<FabricOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn for_flavor(flavor: FabricFlavor) -> Result<Self> {
        let spec = super::flavors::flavor_spec(&flavor);
        let policy = OriginPolicy::default()
            .with_override(FabricOrigin::Meta, spec.meta_origin().to_owned())?
            .with_override(FabricOrigin::Maven, spec.maven_origin().to_owned())?;
        Ok(Self::new(policy))
    }

    pub fn with_overrides(overrides: FabricEndpointOverrides) -> Result<Self> {
        let policy = OriginPolicy::default()
            .with_override(FabricOrigin::Meta, overrides.meta_origin)?
            .with_override(FabricOrigin::Maven, overrides.maven_origin)?;
        Ok(Self::new(policy))
    }

    pub fn origin_policy(&self) -> &OriginPolicy<FabricOrigin> {
        &self.origin_policy
    }

    pub fn game_versions_url(&self) -> Result<String> {
        self.resolve_meta_segments(["v2", "versions", "game"])
    }

    pub fn loader_versions_url(&self, game_version: &str) -> Result<String> {
        self.resolve_meta_segments(["v2", "versions", "loader", game_version])
    }

    pub fn loader_profile_url(&self, game_version: &str, loader_version: &str) -> Result<String> {
        self.resolve_meta_segments(["v2", "versions", "loader", game_version, loader_version])
    }

    pub fn profile_json_url(&self, game_version: &str, loader_version: &str) -> Result<String> {
        self.resolve_meta_segments([
            "v2",
            "versions",
            "loader",
            game_version,
            loader_version,
            "profile",
            "json",
        ])
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        Ok(raw_url.to_owned())
    }

    fn resolve_meta_segments<const N: usize>(&self, segments: [&str; N]) -> Result<String> {
        self.resolve_segments(FabricOrigin::Meta, segments)
    }

    fn resolve_segments<const N: usize>(
        &self,
        origin: FabricOrigin,
        segments: [&str; N],
    ) -> Result<String> {
        let mut url = self.origin_policy.base_url(origin)?;
        {
            let mut path_segments = url
                .path_segments_mut()
                .map_err(|_| anyhow::anyhow!("origin url cannot be used as a path base"))?;
            for segment in segments {
                path_segments.push(segment);
            }
        }

        Ok(url.to_string())
    }
}

impl Default for FabricSource {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent(format!("Elemental/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("build fabric source client failed"),
            endpoints: FabricEndpoints::default(),
        }
    }
}

impl FabricSource {
    pub fn new(endpoints: FabricEndpoints) -> Self {
        Self {
            endpoints,
            ..Self::default()
        }
    }

    pub fn for_flavor(flavor: FabricFlavor) -> Result<Self> {
        Ok(Self::new(FabricEndpoints::for_flavor(flavor)?))
    }

    pub fn with_overrides(overrides: FabricEndpointOverrides) -> Result<Self> {
        Ok(Self::new(FabricEndpoints::with_overrides(overrides)?))
    }

    pub fn with_client(endpoints: FabricEndpoints, client: reqwest::Client) -> Self {
        Self { client, endpoints }
    }

    pub fn endpoints(&self) -> &FabricEndpoints {
        &self.endpoints
    }

    pub async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        let url = self.endpoints.game_versions_url()?;
        self.fetch_json(url.as_str()).await
    }

    pub async fn loader_versions(&self, game_version: &str) -> Result<Vec<LoaderGameVersion>> {
        let url = self.endpoints.loader_versions_url(game_version)?;
        self.fetch_json(url.as_str()).await
    }

    pub async fn loader_profile(
        &self,
        game_version: &str,
        loader_version: &str,
    ) -> Result<LoaderProfile> {
        let url = self
            .endpoints
            .loader_profile_url(game_version, loader_version)?;
        self.fetch_json(url.as_str()).await
    }

    pub async fn profile_json(
        &self,
        game_version: &str,
        loader_version: &str,
    ) -> Result<ProfileJson> {
        let url = self
            .endpoints
            .profile_json_url(game_version, loader_version)?;
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
            .with_context(|| format!("request fabric source resource failed: {url}"))?
            .error_for_status()
            .with_context(|| format!("fabric source returned error status: {url}"))?
            .json::<T>()
            .await
            .with_context(|| format!("decode fabric source resource failed: {url}"))
    }
}
