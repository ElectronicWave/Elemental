use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::fabric::{GameVersion, LoaderProfile, LoaderVersion, ProfileJson};
use serde::Deserialize;

use crate::{
    error::Result,
    families::version_json::{
        LoaderMetaEndpoints, LoaderMetaSource, LoaderProfileEndpoints, UpstreamUrlRewriter,
    },
    loader_version::LoaderVersionId,
    url::{Origin, OriginEndpoints, OriginPolicy},
};

pub use super::flavors::FabricFlavor;

const KNOWN_FABRIC_ORIGINS: &[&str] = &[
    "https://meta.fabricmc.net",
    "https://maven.fabricmc.net",
    "https://meta.legacyfabric.net",
    "https://maven.legacyfabric.net",
    "https://meta.babric.glass-launcher.net",
    "https://maven.glass-launcher.net/babric",
];

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

impl AsRef<str> for FabricOrigin {
    fn as_ref(&self) -> &str {
        self.canonical()
    }
}

#[derive(Debug, Clone)]
pub struct FabricEndpoints {
    endpoints: OriginEndpoints,
}

impl Default for FabricEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

pub type FabricSource = LoaderMetaSource<FabricEndpoints>;

#[derive(Debug, Clone, Deserialize)]
pub struct FabricLoaderListEntry {
    pub loader: LoaderVersion,
}

impl FabricEndpoints {
    pub fn new(origin_policy: OriginPolicy) -> Self {
        Self {
            endpoints: OriginEndpoints::new(KNOWN_FABRIC_ORIGINS, origin_policy),
        }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn origin_policy(&self) -> &OriginPolicy {
        self.endpoints.origin_policy()
    }

    pub fn for_flavor(flavor: FabricFlavor) -> Result<Self> {
        Self::for_flavor_with_policy(flavor, &OriginPolicy::default())
    }

    pub fn for_flavor_with_policy(flavor: FabricFlavor, policy: &OriginPolicy) -> Result<Self> {
        let spec = super::flavors::flavor_spec(&flavor);
        let meta_override = policy
            .override_for(spec.meta_origin())
            .or_else(|| policy.override_for(FabricOrigin::Meta.canonical()))
            .map_or_else(|| spec.meta_origin().to_owned(), |url| url.to_string());
        let maven_override = policy
            .override_for(spec.maven_origin())
            .or_else(|| policy.override_for(FabricOrigin::Maven.canonical()))
            .map_or_else(|| spec.maven_origin().to_owned(), |url| url.to_string());

        let mut merged = policy.clone();
        merged.override_origin(FabricOrigin::Meta, meta_override)?;
        merged.override_origin(FabricOrigin::Maven, maven_override)?;
        Ok(Self::new(merged))
    }

    pub fn with_endpoint_overrides(overrides: FabricEndpointOverrides) -> Result<Self> {
        Ok(Self {
            endpoints: OriginEndpoints::with_overrides(
                KNOWN_FABRIC_ORIGINS,
                [
                    (FabricOrigin::Meta, overrides.meta_origin),
                    (FabricOrigin::Maven, overrides.maven_origin),
                ],
            )?,
        })
    }

    pub fn game_versions_url(&self) -> Result<String> {
        self.resolve_meta_segments(&["v2", "versions", "game"])
    }

    pub fn loader_versions_url(&self, game_version: &str) -> Result<String> {
        self.resolve_meta_segments(&["v2", "versions", "loader", game_version])
    }

    pub fn loader_profile_url(&self, game_version: &str, loader_version: &str) -> Result<String> {
        self.resolve_meta_segments(&["v2", "versions", "loader", game_version, loader_version])
    }

    pub fn profile_json_url(&self, game_version: &str, loader_version: &str) -> Result<String> {
        self.resolve_meta_segments(&[
            "v2",
            "versions",
            "loader",
            game_version,
            loader_version,
            "profile",
            "json",
        ])
    }

    fn resolve_meta_segments(&self, segments: &[&str]) -> Result<String> {
        self.endpoints
            .resolve_segments(FabricOrigin::Meta, segments)
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
            channel: Some(if version.stable { "stable" } else { "snapshot" }.to_owned()),
        }
    }

    fn release(
        game_version: MinecraftVersionId,
        version: Self::LoaderGameVersion,
    ) -> crate::catalog::LoaderRelease {
        let channel = if version.loader.stable {
            "stable"
        } else {
            "unstable"
        };
        crate::catalog::LoaderRelease::new(
            game_version,
            LoaderVersionId::from(version.loader.version),
        )
        .with_channel(Some(channel.to_owned()))
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
