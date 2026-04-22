use std::path::PathBuf;

use anyhow::Result;
use elemental_core::{minecraft::MinecraftVersionId, storage::Storage};
use elemental_schema::forge::MavenMetadataBody;

use crate::{
    families::{
        installer::{
            InstallerArtifact, InstallerArtifactEndpoints, InstallerArtifactSource,
            build_installer_artifact,
        },
        version_json::VersionJsonRootLayout,
    },
    http::build_default_client,
    loader_version::LoaderVersionId,
    maven::fetch_maven_metadata,
    url::{Origin, OriginPolicy},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CleanroomOrigin {
    Maven,
}

#[derive(Debug, Clone)]
pub struct CleanroomEndpoints {
    origin_policy: OriginPolicy<CleanroomOrigin>,
}

#[derive(Debug, Clone)]
pub struct CleanroomSource {
    client: reqwest::Client,
    endpoints: CleanroomEndpoints,
}

impl Origin for CleanroomOrigin {
    fn canonical(self) -> &'static str {
        match self {
            Self::Maven => "https://repo.cleanroommc.com/releases",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[CleanroomOrigin] = &[CleanroomOrigin::Maven];
        ALL
    }
}

impl Default for CleanroomEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl CleanroomEndpoints {
    pub fn new(origin_policy: OriginPolicy<CleanroomOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn origin_policy(&self) -> &OriginPolicy<CleanroomOrigin> {
        &self.origin_policy
    }

    pub fn maven_metadata_url(&self) -> Result<String> {
        self.origin_policy.resolve(
            CleanroomOrigin::Maven,
            "/com/cleanroommc/cleanroom/maven-metadata.xml",
        )
    }

    pub fn installer_url(&self, loader_version: &str) -> Result<String> {
        let version = release_version(loader_version);
        self.origin_policy.resolve(
            CleanroomOrigin::Maven,
            &format!("/com/cleanroommc/cleanroom/{version}/cleanroom-{version}-installer.jar"),
        )
    }

    pub fn maven_artifact_url(&self, artifact_path: &str) -> Result<String> {
        self.origin_policy
            .resolve(CleanroomOrigin::Maven, artifact_path)
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        if let Some(rewritten) = self.origin_policy.rewrite_known_origin_url(raw_url)? {
            return Ok(rewritten);
        }

        Ok(raw_url.to_owned())
    }
}

impl InstallerArtifactEndpoints for CleanroomEndpoints {
    fn artifact_url(&self, artifact_path: &str) -> Result<String> {
        CleanroomEndpoints::maven_artifact_url(self, artifact_path)
    }

    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        CleanroomEndpoints::rewrite_upstream(self, raw_url)
    }
}

impl Default for CleanroomSource {
    fn default() -> Self {
        Self {
            client: build_default_client("cleanroom source"),
            endpoints: CleanroomEndpoints::default(),
        }
    }
}

impl CleanroomSource {
    pub fn new(endpoints: CleanroomEndpoints) -> Self {
        Self {
            endpoints,
            ..Self::default()
        }
    }

    pub fn endpoints(&self) -> &CleanroomEndpoints {
        &self.endpoints
    }

    pub async fn maven_metadata(&self) -> Result<MavenMetadataBody> {
        let url = self.endpoints.maven_metadata_url()?;
        fetch_maven_metadata(&self.client, url, "cleanroom source").await
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
        let library_relative_path = cleanroom_installer_relative_path(&version);

        build_installer_artifact(
            game_storage,
            format!("com.cleanroommc:cleanroom:{version}:installer"),
            self.endpoints.installer_url(loader_version.as_str())?,
            library_relative_path,
        )
    }
}

impl InstallerArtifactSource for CleanroomSource {
    type Endpoints = CleanroomEndpoints;

    fn endpoints(&self) -> &Self::Endpoints {
        CleanroomSource::endpoints(self)
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
        CleanroomSource::installer_artifact(self, game_storage, game_version, loader_version)
    }
}

pub fn release_version(loader_version: &str) -> String {
    loader_version.to_owned()
}

pub fn cleanroom_installer_relative_path(version: &str) -> PathBuf {
    PathBuf::from("com")
        .join("cleanroommc")
        .join("cleanroom")
        .join(version)
        .join(format!("cleanroom-{version}-installer.jar"))
}
