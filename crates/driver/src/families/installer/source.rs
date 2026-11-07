mod client;
mod endpoints;
mod spec;

use std::fmt::Debug;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_infra::SafeRelativePath;

use crate::{
    catalog::LoaderRelease, error::Result, families::installer::InstallerArtifactEndpoints,
    loader_version::LoaderVersionId,
};

pub use client::InstallerMavenSource;
pub use endpoints::MavenEndpoints;
pub use spec::MavenSpec;

#[derive(Debug, Clone)]
pub struct InstallerMavenArtifactSpec {
    pub coordinate: String,
    pub download_url: String,
    pub relative_path: SafeRelativePath,
}

pub trait InstallerMavenEndpoints:
    InstallerArtifactEndpoints + Clone + Debug + Send + Sync + 'static
{
    const SOURCE_NAME: &'static str;

    fn maven_metadata_url(&self) -> Result<String>;

    fn installer_artifact_spec(
        &self,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec>;

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)>;
}
