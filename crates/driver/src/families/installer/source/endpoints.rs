use elemental_core::minecraft::MinecraftVersionId;
use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::PistonMetaLibraries;

use crate::{
    error::Result,
    families::installer::{InstallerMavenArtifactSpec, MavenInstallerSpec, normalize_library_urls},
    loader_version::LoaderVersionId,
    url::OriginPolicy,
};

#[derive(Debug, Clone)]
pub struct MavenEndpoints {
    spec: &'static MavenInstallerSpec,
    known_origins: [&'static str; 1],
    origin_policy: OriginPolicy,
}

impl MavenEndpoints {
    pub fn new(spec: &'static MavenInstallerSpec, origin_policy: OriginPolicy) -> Self {
        Self {
            spec,
            known_origins: [spec.canonical_origin],
            origin_policy,
        }
    }

    pub fn official(spec: &'static MavenInstallerSpec) -> Self {
        Self::new(spec, OriginPolicy::default())
    }

    pub fn spec(&self) -> &'static MavenInstallerSpec {
        self.spec
    }

    pub fn resolve(&self, path: &str) -> Result<String> {
        Ok(self
            .origin_policy
            .resolve(self.spec.canonical_origin, path)?)
    }

    pub fn origin_policy(&self) -> &OriginPolicy {
        &self.origin_policy
    }

    pub fn artifact_url(&self, artifact_path: &str) -> Result<String> {
        self.resolve(artifact_path)
    }

    pub fn normalize_libraries(
        &self,
        libraries: Vec<PistonMetaLibraries>,
    ) -> Result<Vec<PistonMetaLibraries>> {
        normalize_library_urls(libraries, |artifact_path| self.artifact_url(artifact_path))
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        Ok(self
            .origin_policy
            .rewrite_known_origin_url_or_keep(&self.known_origins, raw_url)?)
    }

    pub fn maven_metadata_url(&self) -> Result<String> {
        self.resolve(&self.spec.metadata_path()?)
    }

    pub fn installer_artifact_spec(
        &self,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec> {
        let version = self.spec.version(game_version, loader_version);
        let coordinate = self.spec.installer_coordinate(&version)?;
        let artifact_path = coordinate.artifact_path();
        Ok(InstallerMavenArtifactSpec {
            coordinate: coordinate.to_string(),
            download_url: self.resolve(&artifact_path)?,
            relative_path: SafeRelativePath::parse(artifact_path)?,
        })
    }
}
