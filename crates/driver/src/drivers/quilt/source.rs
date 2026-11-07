use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::quilt::{GameVersion, ProfileJson};
use serde::Deserialize;

use crate::{
    error::Result,
    families::version_json::{LoaderMetaEndpoints, LoaderMetaSource, UpstreamUrlRewriter},
    loader_version::LoaderVersionId,
    url::{Origin, OriginEndpoints},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuiltOrigin {
    Meta,
    Maven,
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

pub type QuiltEndpoints = OriginEndpoints<QuiltOrigin>;
pub type QuiltSource = LoaderMetaSource<QuiltEndpoints>;

#[derive(Debug, Clone, Deserialize)]
pub struct QuiltLoaderListEntry {
    pub loader: QuiltLoaderListVersion,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuiltLoaderListVersion {
    pub version: String,
}

impl QuiltEndpoints {
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

    fn resolve_meta_segments<const N: usize>(&self, segments: [&str; N]) -> Result<String> {
        self.resolve_segments(QuiltOrigin::Meta, segments)
    }
}

impl LoaderMetaEndpoints for QuiltEndpoints {
    type GameVersion = GameVersion;
    type LoaderGameVersion = QuiltLoaderListEntry;
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

    fn game_version(version: Self::GameVersion) -> crate::catalog::GameVersion {
        crate::catalog::GameVersion {
            id: MinecraftVersionId::from(version.version),
            description: None,
            stream: Some(if version.stable { "stable" } else { "snapshot" }.to_owned()),
        }
    }

    fn release(
        game_version: MinecraftVersionId,
        version: Self::LoaderGameVersion,
    ) -> crate::catalog::LoaderRelease {
        crate::catalog::LoaderRelease::new(
            game_version,
            LoaderVersionId::from(version.loader.version),
        )
    }
}

impl UpstreamUrlRewriter for QuiltEndpoints {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.rewrite_known_or_keep(raw_url)
    }
}
