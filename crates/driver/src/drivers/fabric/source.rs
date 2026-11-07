use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::fabric::{GameVersion, LoaderProfile, LoaderVersion, ProfileJson};
use serde::Deserialize;

use crate::{
    error::Result,
    families::version_json::{
        LoaderMetaEndpoints, LoaderMetaSource, LoaderProfileEndpoints, UpstreamUrlRewriter,
    },
    loader_version::LoaderVersionId,
    url::{Origin, OriginEndpoints},
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

pub type FabricEndpoints = OriginEndpoints<FabricOrigin>;
pub type FabricSource = LoaderMetaSource<FabricEndpoints>;

#[derive(Debug, Clone, Deserialize)]
pub struct FabricLoaderListEntry {
    pub loader: LoaderVersion,
}

impl FabricEndpoints {
    pub fn for_flavor(flavor: FabricFlavor) -> Result<Self> {
        let spec = super::flavors::flavor_spec(&flavor);
        OriginEndpoints::with_overrides([
            (FabricOrigin::Meta, spec.meta_origin().to_owned()),
            (FabricOrigin::Maven, spec.maven_origin().to_owned()),
        ])
    }

    pub fn with_endpoint_overrides(overrides: FabricEndpointOverrides) -> Result<Self> {
        OriginEndpoints::with_overrides([
            (FabricOrigin::Meta, overrides.meta_origin),
            (FabricOrigin::Maven, overrides.maven_origin),
        ])
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

    fn resolve_meta_segments<const N: usize>(&self, segments: [&str; N]) -> Result<String> {
        self.resolve_segments(FabricOrigin::Meta, segments)
    }
}

impl LoaderMetaEndpoints for FabricEndpoints {
    type GameVersion = GameVersion;
    type LoaderGameVersion = FabricLoaderListEntry;
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
        .with_description(Some(
            if version.loader.stable {
                "Stable"
            } else {
                "Unstable"
            }
            .to_owned(),
        ))
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
        Ok(raw_url.to_owned())
    }
}
