use anyhow::Result;
use elemental_schema::quilt::{GameVersion, LoaderGameVersion, ProfileJson};

use crate::{
    http::{build_default_client, fetch_json},
    url::{Origin, OriginPolicy},
};

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
        self.origin_policy
            .resolve_segments(QuiltOrigin::Meta, segments)
    }
}

impl Default for QuiltSource {
    fn default() -> Self {
        Self {
            client: build_default_client("quilt source"),
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
        fetch_json(&self.client, url.as_str(), "quilt source").await
    }

    pub async fn loader_versions(&self, game_version: &str) -> Result<Vec<LoaderGameVersion>> {
        let url = self.endpoints.loader_versions_url(game_version)?;
        fetch_json(&self.client, url.as_str(), "quilt source").await
    }

    pub async fn profile_json(
        &self,
        game_version: &str,
        loader_version: &str,
    ) -> Result<ProfileJson> {
        let url = self
            .endpoints
            .profile_json_url(game_version, loader_version)?;
        fetch_json(&self.client, url.as_str(), "quilt source").await
    }
}
