use std::path::PathBuf;

use anyhow::{Context, Result};
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
pub enum ForgeOrigin {
    Maven,
}

#[derive(Debug, Clone)]
pub struct ForgeEndpoints {
    origin_policy: OriginPolicy<ForgeOrigin>,
}

pub type ForgeSource = InstallerMavenSource<ForgeEndpoints>;

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

impl InstallerMavenEndpoints for ForgeEndpoints {
    const SOURCE_NAME: &'static str = "forge source";

    fn maven_metadata_url(&self) -> Result<String> {
        ForgeEndpoints::maven_metadata_url(self)
    }

    fn installer_artifact_spec(
        &self,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec> {
        let version = release_version(game_version.as_str(), loader_version.as_str());

        Ok(InstallerMavenArtifactSpec {
            coordinate: format!("net.minecraftforge:forge:{version}:installer"),
            download_url: self.installer_url(game_version.as_str(), loader_version.as_str())?,
            relative_path: forge_installer_relative_path(&version),
        })
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
