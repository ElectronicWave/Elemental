use anyhow::Result;
use elemental_schema::quilt::{GameVersion, LoaderGameVersion, ProfileJson};

use crate::{
    families::version_json::{LoaderMetaEndpoints, LoaderMetaSource, UpstreamUrlRewriter},
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

pub type QuiltSource = LoaderMetaSource<QuiltEndpoints>;

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

impl LoaderMetaEndpoints for QuiltEndpoints {
    type GameVersion = GameVersion;
    type LoaderGameVersion = LoaderGameVersion;
    type ProfileJson = ProfileJson;

    const SOURCE_NAME: &'static str = "quilt source";

    fn game_versions_url(&self) -> Result<String> {
        QuiltEndpoints::game_versions_url(self)
    }

    fn loader_versions_url(&self, game_version: &str) -> Result<String> {
        QuiltEndpoints::loader_versions_url(self, game_version)
    }

    fn profile_json_url(&self, game_version: &str, loader_version: &str) -> Result<String> {
        QuiltEndpoints::profile_json_url(self, game_version, loader_version)
    }
}

impl UpstreamUrlRewriter for QuiltEndpoints {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        QuiltEndpoints::rewrite_upstream(self, raw_url)
    }
}
