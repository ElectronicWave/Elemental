use anyhow::Result;
use elemental_schema::fabric::{GameVersion, LoaderGameVersion, LoaderProfile, ProfileJson};

use crate::{
    families::version_json::{
        LoaderMetaEndpoints, LoaderMetaSource, LoaderProfileEndpoints, UpstreamUrlRewriter,
    },
    url::{Origin, OriginPolicy},
};

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

pub type FabricSource = LoaderMetaSource<FabricEndpoints>;

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
        self.origin_policy
            .resolve_segments(FabricOrigin::Meta, segments)
    }
}

impl LoaderMetaEndpoints for FabricEndpoints {
    type GameVersion = GameVersion;
    type LoaderGameVersion = LoaderGameVersion;
    type ProfileJson = ProfileJson;

    const SOURCE_NAME: &'static str = "fabric source";

    fn game_versions_url(&self) -> Result<String> {
        FabricEndpoints::game_versions_url(self)
    }

    fn loader_versions_url(&self, game_version: &str) -> Result<String> {
        FabricEndpoints::loader_versions_url(self, game_version)
    }

    fn profile_json_url(&self, game_version: &str, loader_version: &str) -> Result<String> {
        FabricEndpoints::profile_json_url(self, game_version, loader_version)
    }
}

impl LoaderProfileEndpoints for FabricEndpoints {
    type LoaderProfile = LoaderProfile;

    fn loader_profile_url(&self, game_version: &str, loader_version: &str) -> Result<String> {
        FabricEndpoints::loader_profile_url(self, game_version, loader_version)
    }
}

impl UpstreamUrlRewriter for FabricEndpoints {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        FabricEndpoints::rewrite_upstream(self, raw_url)
    }
}
