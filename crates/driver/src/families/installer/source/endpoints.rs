use std::marker::PhantomData;

use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    catalog::LoaderRelease,
    error::Result,
    families::installer::{
        InstallerArtifactEndpoints, InstallerMavenArtifactSpec, InstallerMavenEndpoints,
    },
    loader_version::LoaderVersionId,
    url::{OriginEndpoints, OriginPolicy},
};

use super::spec::MavenSpec;

#[derive(Debug, Clone)]
pub struct MavenEndpoints<S>
where
    S: MavenSpec,
{
    endpoints: OriginEndpoints<S::Origin>,
    marker: PhantomData<S>,
}

impl<S> Default for MavenEndpoints<S>
where
    S: MavenSpec,
{
    fn default() -> Self {
        Self::official()
    }
}

impl<S> MavenEndpoints<S>
where
    S: MavenSpec,
{
    pub fn new(endpoints: OriginEndpoints<S::Origin>) -> Self {
        Self {
            endpoints,
            marker: PhantomData,
        }
    }

    pub fn official() -> Self {
        Self::new(OriginEndpoints::official())
    }

    pub fn resolve(&self, path: &str) -> Result<String> {
        self.endpoints.resolve(S::MAVEN_ORIGIN, path)
    }

    pub fn origin_policy(&self) -> &OriginPolicy<S::Origin> {
        self.endpoints.origin_policy()
    }
}

impl<S> InstallerArtifactEndpoints for MavenEndpoints<S>
where
    S: MavenSpec,
{
    fn artifact_url(&self, artifact_path: &str) -> Result<String> {
        self.resolve(artifact_path)
    }

    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        S::rewrite_upstream(&self.endpoints, raw_url)
    }
}

impl<S> InstallerMavenEndpoints for MavenEndpoints<S>
where
    S: MavenSpec,
{
    const SOURCE_NAME: &'static str = S::SOURCE_NAME;

    fn maven_metadata_url(&self) -> Result<String> {
        self.resolve(&S::metadata_path()?)
    }

    fn installer_artifact_spec(
        &self,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec> {
        S::installer_artifact(&self.endpoints, game_version, loader_version)
    }

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)> {
        S::release(version)
    }
}
