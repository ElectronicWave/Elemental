use std::time::Duration;

use anyhow::{Context, Result};
use elemental_schema::quilt::{GameVersion, LoaderGameVersion, ProfileJson};
use serde::de::DeserializeOwned;

use crate::url::{Origin, OriginPolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuiltOrigin {
    Meta,
    Maven,
}

#[derive(Debug, Clone)]
pub struct QuiltEndpoints {
    origin_policy: OriginPolicy<QuiltOrigin>,
}

#[derive(Debug, Clone)]
pub struct QuiltSource {
    client: reqwest::Client,
    endpoints: QuiltEndpoints,
}

impl Default for QuiltEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl Origin for QuiltOrigin {
    fn canonical(self) -> &'static str {
        match self {
            QuiltOrigin::Meta => "https://meta.quiltmc.org",
            QuiltOrigin::Maven => "https://maven.quiltmc.org/repository/release",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[QuiltOrigin] = &[QuiltOrigin::Meta, QuiltOrigin::Maven];
        ALL
    }
}

impl QuiltEndpoints {
    pub fn new(origin_policy: OriginPolicy<QuiltOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn origin_policy(&self) -> &OriginPolicy<QuiltOrigin> {
        &self.origin_policy
    }

    pub fn game_versions_url(&self) -> Result<String> {
        self.resolve_meta_segments(["v3", "versions", "game"])
    }

    pub fn loader_versions_url(&self, game_version: &str) -> Result<String> {
        self.resolve_meta_segments(["v3", "versions", "loader", game_version])
    }

    pub fn profile_json_url(&self, game_version: &str, loader_version: &str) -> Result<String> {
        self.resolve_meta_segments([
            "v3",
            "versions",
            "loader",
            game_version,
            loader_version,
            "profile",
            "json",
        ])
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        if let Some(rewritten) = self.origin_policy.rewrite_known_origin_url(raw_url)? {
            return Ok(rewritten);
        }

        Ok(raw_url.to_owned())
    }

    fn resolve_meta_segments<const N: usize>(&self, segments: [&str; N]) -> Result<String> {
        self.resolve_segments(QuiltOrigin::Meta, segments)
    }

    fn resolve_segments<const N: usize>(
        &self,
        origin: QuiltOrigin,
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

impl Default for QuiltSource {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent(format!("Elemental/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("build quilt source client failed"),
            endpoints: QuiltEndpoints::default(),
        }
    }
}

impl QuiltSource {
    pub fn new(endpoints: QuiltEndpoints) -> Self {
        Self {
            endpoints,
            ..Self::default()
        }
    }

    pub fn endpoints(&self) -> &QuiltEndpoints {
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
            .with_context(|| format!("request quilt source resource failed: {url}"))?
            .error_for_status()
            .with_context(|| format!("quilt source returned error status: {url}"))?
            .json::<T>()
            .await
            .with_context(|| format!("decode quilt source resource failed: {url}"))
    }
}
