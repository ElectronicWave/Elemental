use std::path::PathBuf;

use anyhow::Result;
use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    families::installer::{
        InstallerArtifactEndpoints, InstallerMavenArtifactSpec, InstallerMavenEndpoints,
        InstallerMavenSource,
    },
    loader_version::LoaderVersionId,
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

pub type CleanroomSource = InstallerMavenSource<CleanroomEndpoints>;

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

impl InstallerMavenEndpoints for CleanroomEndpoints {
    const SOURCE_NAME: &'static str = "cleanroom source";

    fn maven_metadata_url(&self) -> Result<String> {
        CleanroomEndpoints::maven_metadata_url(self)
    }

    fn installer_artifact_spec(
        &self,
        _game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec> {
        let version = release_version(loader_version.as_str());

        Ok(InstallerMavenArtifactSpec {
            coordinate: format!("com.cleanroommc:cleanroom:{version}:installer"),
            download_url: self.installer_url(loader_version.as_str())?,
            relative_path: cleanroom_installer_relative_path(&version),
        })
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
