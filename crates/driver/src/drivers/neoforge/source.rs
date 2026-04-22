use std::path::PathBuf;

use crate::{
    families::{
        installer::{
            InstallerArtifact, InstallerArtifactEndpoints, InstallerArtifactSource,
            build_installer_artifact,
        },
        version_json::VersionJsonRootLayout,
    },
    http::HttpSource,
    loader_version::LoaderVersionId,
    maven::fetch_maven_metadata,
    url::{Origin, OriginPolicy},
};
use anyhow::Result;
use elemental_core::{minecraft::MinecraftVersionId, storage::Storage};
use elemental_schema::forge::MavenMetadataBody;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NeoForgeOrigin {
    Maven,
}

#[derive(Debug, Clone)]
pub struct NeoForgeEndpoints {
    origin_policy: OriginPolicy<NeoForgeOrigin>,
}

#[derive(Debug, Clone)]
pub struct NeoForgeSource {
    inner: HttpSource<NeoForgeEndpoints>,
}

impl Origin for NeoForgeOrigin {
    fn canonical(self) -> &'static str {
        match self {
            Self::Maven => "https://maven.neoforged.net/releases",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[NeoForgeOrigin] = &[NeoForgeOrigin::Maven];
        ALL
    }
}

impl Default for NeoForgeEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl NeoForgeEndpoints {
    pub fn new(origin_policy: OriginPolicy<NeoForgeOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn origin_policy(&self) -> &OriginPolicy<NeoForgeOrigin> {
        &self.origin_policy
    }

    pub fn maven_metadata_url(&self) -> Result<String> {
        self.origin_policy.resolve(
            NeoForgeOrigin::Maven,
            "/net/neoforged/neoforge/maven-metadata.xml",
        )
    }

    pub fn installer_url(&self, loader_version: &str) -> Result<String> {
        let version = release_version(loader_version);
        self.origin_policy.resolve(
            NeoForgeOrigin::Maven,
            &format!("/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar"),
        )
    }

    pub fn maven_artifact_url(&self, artifact_path: &str) -> Result<String> {
        self.origin_policy
            .resolve(NeoForgeOrigin::Maven, artifact_path)
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        if let Some(rewritten) = self.origin_policy.rewrite_known_origin_url(raw_url)? {
            return Ok(rewritten);
        }

        Ok(raw_url.to_owned())
    }
}

impl InstallerArtifactEndpoints for NeoForgeEndpoints {
    fn artifact_url(&self, artifact_path: &str) -> Result<String> {
        NeoForgeEndpoints::maven_artifact_url(self, artifact_path)
    }

    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        NeoForgeEndpoints::rewrite_upstream(self, raw_url)
    }
}

impl Default for NeoForgeSource {
    fn default() -> Self {
        Self::new(NeoForgeEndpoints::default())
    }
}

impl NeoForgeSource {
    pub fn new(endpoints: NeoForgeEndpoints) -> Self {
        Self {
            inner: HttpSource::new(endpoints, "neoforge source"),
        }
    }

    pub fn with_client(endpoints: NeoForgeEndpoints, client: reqwest::Client) -> Self {
        Self {
            inner: HttpSource::with_client(endpoints, client),
        }
    }

    pub fn endpoints(&self) -> &NeoForgeEndpoints {
        self.inner.endpoints()
    }

    pub async fn maven_metadata(&self) -> Result<MavenMetadataBody> {
        let url = self.endpoints().maven_metadata_url()?;
        fetch_maven_metadata(self.inner.client(), url, "neoforge source").await
    }

    pub fn installer_artifact<L>(
        &self,
        game_storage: &Storage<L>,
        _game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerArtifact>
    where
        L: VersionJsonRootLayout,
    {
        let version = release_version(loader_version.as_str());
        let library_relative_path = neoforge_installer_relative_path(&version);

        build_installer_artifact(
            game_storage,
            format!("net.neoforged:neoforge:{version}:installer"),
            self.endpoints().installer_url(loader_version.as_str())?,
            library_relative_path,
        )
    }
}

impl InstallerArtifactSource for NeoForgeSource {
    type Endpoints = NeoForgeEndpoints;

    fn endpoints(&self) -> &Self::Endpoints {
        NeoForgeSource::endpoints(self)
    }

    fn installer_artifact<L>(
        &self,
        game_storage: &Storage<L>,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerArtifact>
    where
        L: VersionJsonRootLayout,
    {
        NeoForgeSource::installer_artifact(self, game_storage, game_version, loader_version)
    }
}

pub fn release_version(loader_version: &str) -> String {
    loader_version.to_owned()
}

pub fn neoforge_installer_relative_path(version: &str) -> PathBuf {
    PathBuf::from("net")
        .join("neoforged")
        .join("neoforge")
        .join(version)
        .join(format!("neoforge-{version}-installer.jar"))
}
