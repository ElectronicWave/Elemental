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
use anyhow::{Context, Result};
use elemental_core::{minecraft::MinecraftVersionId, storage::Storage};
use elemental_schema::forge::MavenMetadataBody;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForgeOrigin {
    Maven,
}

#[derive(Debug, Clone)]
pub struct ForgeEndpoints {
    origin_policy: OriginPolicy<ForgeOrigin>,
}

#[derive(Debug, Clone)]
pub struct ForgeSource {
    inner: HttpSource<ForgeEndpoints>,
}

impl Origin for ForgeOrigin {
    fn canonical(self) -> &'static str {
        match self {
            Self::Maven => "https://maven.minecraftforge.net",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[ForgeOrigin] = &[ForgeOrigin::Maven];
        ALL
    }
}

impl Default for ForgeEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl ForgeEndpoints {
    pub fn new(origin_policy: OriginPolicy<ForgeOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn origin_policy(&self) -> &OriginPolicy<ForgeOrigin> {
        &self.origin_policy
    }

    pub fn maven_metadata_url(&self) -> Result<String> {
        self.origin_policy.resolve(
            ForgeOrigin::Maven,
            "/net/minecraftforge/forge/maven-metadata.xml",
        )
    }

    pub fn installer_url(&self, game_version: &str, loader_version: &str) -> Result<String> {
        let version = release_version(game_version, loader_version);
        self.origin_policy.resolve(
            ForgeOrigin::Maven,
            &format!("/net/minecraftforge/forge/{version}/forge-{version}-installer.jar"),
        )
    }

    pub fn maven_artifact_url(&self, artifact_path: &str) -> Result<String> {
        self.origin_policy
            .resolve(ForgeOrigin::Maven, artifact_path)
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.origin_policy.rewrite_origin_url(raw_url)
    }
}

impl InstallerArtifactEndpoints for ForgeEndpoints {
    fn artifact_url(&self, artifact_path: &str) -> Result<String> {
        ForgeEndpoints::maven_artifact_url(self, artifact_path)
    }

    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        ForgeEndpoints::rewrite_upstream(self, raw_url)
    }
}

impl Default for ForgeSource {
    fn default() -> Self {
        Self::new(ForgeEndpoints::default())
    }
}

impl ForgeSource {
    pub fn new(endpoints: ForgeEndpoints) -> Self {
        Self {
            inner: HttpSource::new(endpoints, "forge source"),
        }
    }

    pub fn with_client(endpoints: ForgeEndpoints, client: reqwest::Client) -> Self {
        Self {
            inner: HttpSource::with_client(endpoints, client),
        }
    }

    pub fn endpoints(&self) -> &ForgeEndpoints {
        self.inner.endpoints()
    }

    pub async fn maven_metadata(&self) -> Result<MavenMetadataBody> {
        let url = self.endpoints().maven_metadata_url()?;
        fetch_maven_metadata(self.inner.client(), url, "forge source").await
    }

    pub fn installer_artifact<L>(
        &self,
        game_storage: &Storage<L>,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerArtifact>
    where
        L: VersionJsonRootLayout,
    {
        let version = release_version(game_version.as_str(), loader_version.as_str());
        let library_relative_path = forge_installer_relative_path(&version);

        build_installer_artifact(
            game_storage,
            format!("net.minecraftforge:forge:{version}:installer"),
            self.endpoints()
                .installer_url(game_version.as_str(), loader_version.as_str())?,
            library_relative_path,
        )
    }
}

impl InstallerArtifactSource for ForgeSource {
    type Endpoints = ForgeEndpoints;

    fn endpoints(&self) -> &Self::Endpoints {
        ForgeSource::endpoints(self)
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
        ForgeSource::installer_artifact(self, game_storage, game_version, loader_version)
    }
}

pub fn release_version(game_version: &str, loader_version: &str) -> String {
    format!("{game_version}-{loader_version}")
}

pub fn forge_installer_relative_path(version: &str) -> PathBuf {
    PathBuf::from("net")
        .join("minecraftforge")
        .join("forge")
        .join(version)
        .join(format!("forge-{version}-installer.jar"))
}

pub fn parse_installer_version(version: &str) -> Result<(String, String)> {
    version
        .split_once('-')
        .map(|(game_version, loader_version)| (game_version.to_owned(), loader_version.to_owned()))
        .context(format!("forge installer version is invalid: {version}"))
}
