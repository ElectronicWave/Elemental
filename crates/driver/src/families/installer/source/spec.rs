use std::fmt::Debug;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_infra::SafeRelativePath;

use crate::{
    catalog::LoaderRelease,
    error::Result,
    families::installer::InstallerMavenArtifactSpec,
    loader_version::LoaderVersionId,
    url::{Origin, OriginEndpoints},
};

pub trait MavenSpec: Clone + Debug + Send + Sync + 'static {
    type Origin: Origin;

    const SOURCE_NAME: &'static str;
    const MAVEN_ORIGIN: Self::Origin;
    const GROUP_PATH: &'static str;
    const COORDINATE_GROUP: &'static str;
    const ARTIFACT: &'static str;

    fn version(game_version: &MinecraftVersionId, loader_version: &LoaderVersionId) -> String {
        format!("{}-{}", game_version.as_str(), loader_version.as_str())
    }

    fn metadata_path() -> String {
        format!(
            "/{}/{}/maven-metadata.xml",
            Self::GROUP_PATH,
            Self::ARTIFACT
        )
    }

    fn artifact_path(version: &str) -> String {
        format!(
            "/{}/{}/{version}/{}-{version}-installer.jar",
            Self::GROUP_PATH,
            Self::ARTIFACT,
            Self::ARTIFACT,
        )
    }

    fn installer_artifact(
        endpoints: &OriginEndpoints<Self::Origin>,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec> {
        let version = Self::version(game_version, loader_version);
        let artifact_path = Self::artifact_path(&version);
        Ok(InstallerMavenArtifactSpec {
            coordinate: format!(
                "{}:{}:{version}:installer",
                Self::COORDINATE_GROUP,
                Self::ARTIFACT
            ),
            download_url: endpoints.resolve(Self::MAVEN_ORIGIN, &artifact_path)?,
            relative_path: SafeRelativePath::parse(artifact_path.trim_start_matches('/'))?,
        })
    }

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)>;

    fn rewrite_upstream(
        endpoints: &OriginEndpoints<Self::Origin>,
        raw_url: &str,
    ) -> Result<String> {
        endpoints.rewrite_known_or_keep(raw_url)
    }
}
