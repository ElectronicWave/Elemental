use std::path::PathBuf;

use crate::error::Result;
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
pub enum OutlandsOrigin {
    Maven,
}

#[derive(Debug, Clone)]
pub struct OutlandsEndpoints {
    origin_policy: OriginPolicy<OutlandsOrigin>,
}

pub type OutlandsSource = InstallerMavenSource<OutlandsEndpoints>;

impl Origin for OutlandsOrigin {
    fn canonical(self) -> &'static str {
        match self {
            Self::Maven => "https://maven.outlands.top/releases/",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[OutlandsOrigin] = &[OutlandsOrigin::Maven];
        ALL
    }
}

impl Default for OutlandsEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl OutlandsEndpoints {
    pub fn new(origin_policy: OriginPolicy<OutlandsOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn origin_policy(&self) -> &OriginPolicy<OutlandsOrigin> {
        &self.origin_policy
    }

    pub fn maven_metadata_url(&self) -> Result<String> {
        self.origin_policy.resolve(
            OutlandsOrigin::Maven,
            "/top/outlands/outlands/maven-metadata.xml",
        )
    }

    pub fn installer_url(&self, loader_version: &str) -> Result<String> {
        let version = release_version(loader_version);
        self.origin_policy.resolve(
            OutlandsOrigin::Maven,
            &format!("/top/outlands/outlands/{version}/outlands-{version}-installer.jar"),
        )
    }

    pub fn maven_artifact_url(&self, artifact_path: &str) -> Result<String> {
        self.origin_policy
            .resolve(OutlandsOrigin::Maven, artifact_path)
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.origin_policy.rewrite_known_origin_url_or_keep(raw_url)
    }
}

impl InstallerArtifactEndpoints for OutlandsEndpoints {
    fn artifact_url(&self, artifact_path: &str) -> Result<String> {
        OutlandsEndpoints::maven_artifact_url(self, artifact_path)
    }

    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        OutlandsEndpoints::rewrite_upstream(self, raw_url)
    }
}

impl InstallerMavenEndpoints for OutlandsEndpoints {
    const SOURCE_NAME: &'static str = "Outlands source";

    fn maven_metadata_url(&self) -> Result<String> {
        OutlandsEndpoints::maven_metadata_url(self)
    }

    fn installer_artifact_spec(
        &self,
        _game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec> {
        let version = release_version(loader_version.as_str());

        Ok(InstallerMavenArtifactSpec {
            coordinate: format!("top.outlands:outlands:{version}:installer"),
            download_url: self.installer_url(loader_version.as_str())?,
            relative_path: outlands_installer_relative_path(&version),
        })
    }
}

pub fn release_version(loader_version: &str) -> String {
    loader_version.to_owned()
}

pub fn outlands_installer_relative_path(version: &str) -> PathBuf {
    PathBuf::from("top")
        .join("outlands")
        .join("outlands")
        .join(version)
        .join(format!("outlands-{version}-installer.jar"))
}
