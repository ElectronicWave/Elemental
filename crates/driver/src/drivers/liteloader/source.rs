use anyhow::Result;
use elemental_schema::fabric::ProfileJson;

use crate::{
    http::{build_default_client, fetch_json},
    url::{Origin, OriginPolicy},
};

pub use super::manifest::LiteLoaderRelease;
use super::{
    manifest::{LiteLoaderManifest, collect_releases, select_build},
    profile::build_profile_json,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteLoaderOrigin {
    Downloads,
    Snapshots,
    SpongeMaven,
    MojangLibraries,
    MavenCentral,
}

#[derive(Debug, Clone)]
pub struct LiteLoaderEndpoints {
    origin_policy: OriginPolicy<LiteLoaderOrigin>,
}

#[derive(Debug, Clone)]
pub struct LiteLoaderSource {
    client: reqwest::Client,
    endpoints: LiteLoaderEndpoints,
}

impl Default for LiteLoaderEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl Origin for LiteLoaderOrigin {
    fn canonical(self) -> &'static str {
        match self {
            Self::Downloads => "https://dl.liteloader.com/versions",
            Self::Snapshots => "https://repo.mumfrey.com/content/repositories/snapshots",
            Self::SpongeMaven => "https://repo.spongepowered.org/maven",
            Self::MojangLibraries => "https://libraries.minecraft.net",
            Self::MavenCentral => "https://repo.maven.apache.org/maven2",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[LiteLoaderOrigin] = &[
            LiteLoaderOrigin::Downloads,
            LiteLoaderOrigin::Snapshots,
            LiteLoaderOrigin::SpongeMaven,
            LiteLoaderOrigin::MojangLibraries,
            LiteLoaderOrigin::MavenCentral,
        ];
        ALL
    }
}

impl LiteLoaderEndpoints {
    pub fn new(origin_policy: OriginPolicy<LiteLoaderOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn versions_manifest_url(&self) -> Result<String> {
        self.origin_policy
            .resolve(LiteLoaderOrigin::Downloads, "versions.json")
    }

    pub fn downloads_url(&self) -> Result<String> {
        self.origin_policy.resolve(LiteLoaderOrigin::Downloads, "")
    }

    pub fn snapshots_url(&self) -> Result<String> {
        self.origin_policy.resolve(LiteLoaderOrigin::Snapshots, "")
    }

    pub fn sponge_maven_url(&self) -> Result<String> {
        self.origin_policy
            .resolve(LiteLoaderOrigin::SpongeMaven, "")
    }

    pub fn mojang_libraries_url(&self) -> Result<String> {
        self.origin_policy
            .resolve(LiteLoaderOrigin::MojangLibraries, "")
    }

    pub fn maven_central_url(&self) -> Result<String> {
        self.origin_policy
            .resolve(LiteLoaderOrigin::MavenCentral, "")
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        if let Some(rewritten) = rewrite_legacy_origin(
            &self.origin_policy,
            raw_url,
            "http://dl.liteloader.com/versions",
            LiteLoaderOrigin::Downloads,
        )? {
            return Ok(rewritten);
        }

        if let Some(rewritten) = rewrite_legacy_origin(
            &self.origin_policy,
            raw_url,
            "http://repo.mumfrey.com/content/repositories/snapshots",
            LiteLoaderOrigin::Snapshots,
        )? {
            return Ok(rewritten);
        }

        if let Some(rewritten) = rewrite_legacy_origin(
            &self.origin_policy,
            raw_url,
            "http://repo.spongepowered.org/maven",
            LiteLoaderOrigin::SpongeMaven,
        )? {
            return Ok(rewritten);
        }

        if let Some(rewritten) = rewrite_legacy_origin(
            &self.origin_policy,
            raw_url,
            "http://repo.maven.apache.org/maven2",
            LiteLoaderOrigin::MavenCentral,
        )? {
            return Ok(rewritten);
        }

        if let Some(rewritten) = rewrite_legacy_origin(
            &self.origin_policy,
            raw_url,
            "http://repo1.maven.org/maven2",
            LiteLoaderOrigin::MavenCentral,
        )? {
            return Ok(rewritten);
        }

        if let Some(rewritten) = self.origin_policy.rewrite_known_origin_url(raw_url)? {
            return Ok(rewritten);
        }

        Ok(raw_url.to_owned())
    }
}

impl Default for LiteLoaderSource {
    fn default() -> Self {
        Self {
            client: build_default_client("liteloader source"),
            endpoints: LiteLoaderEndpoints::default(),
        }
    }
}

impl LiteLoaderSource {
    pub fn new(endpoints: LiteLoaderEndpoints) -> Self {
        Self {
            endpoints,
            ..Self::default()
        }
    }

    pub fn endpoints(&self) -> &LiteLoaderEndpoints {
        &self.endpoints
    }

    pub async fn releases(&self) -> Result<Vec<LiteLoaderRelease>> {
        let manifest = self.manifest().await?;
        Ok(collect_releases(manifest))
    }

    pub async fn profile_json(
        &self,
        game_version: &str,
        loader_version: &str,
    ) -> Result<ProfileJson> {
        let manifest = self.manifest().await?;
        let selected = select_build(&manifest, game_version, loader_version)?;
        build_profile_json(&self.client, &self.endpoints, &selected).await
    }

    async fn manifest(&self) -> Result<LiteLoaderManifest> {
        let url = self.endpoints.versions_manifest_url()?;
        fetch_json(&self.client, url.as_str(), "liteloader source").await
    }
}

fn rewrite_legacy_origin(
    policy: &OriginPolicy<LiteLoaderOrigin>,
    raw_url: &str,
    legacy_base: &str,
    origin: LiteLoaderOrigin,
) -> Result<Option<String>> {
    let trimmed_legacy_base = legacy_base.trim_end_matches('/');
    let Some(suffix) = raw_url.strip_prefix(trimmed_legacy_base) else {
        return Ok(None);
    };

    policy.resolve(origin, suffix).map(Some)
}
