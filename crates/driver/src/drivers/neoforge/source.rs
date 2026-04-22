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
pub enum NeoForgeOrigin {
    Maven,
}

#[derive(Debug, Clone)]
pub struct NeoForgeEndpoints {
    origin_policy: OriginPolicy<NeoForgeOrigin>,
}

pub type NeoForgeSource = InstallerMavenSource<NeoForgeEndpoints>;

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
        self.origin_policy.rewrite_known_origin_url_or_keep(raw_url)
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

impl InstallerMavenEndpoints for NeoForgeEndpoints {
    const SOURCE_NAME: &'static str = "neoforge source";

    fn maven_metadata_url(&self) -> Result<String> {
        NeoForgeEndpoints::maven_metadata_url(self)
    }

    fn installer_artifact_spec(
        &self,
        _game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec> {
        let version = release_version(loader_version.as_str());

        Ok(InstallerMavenArtifactSpec {
            coordinate: format!("net.neoforged:neoforge:{version}:installer"),
            download_url: self.installer_url(loader_version.as_str())?,
            relative_path: neoforge_installer_relative_path(&version),
        })
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
