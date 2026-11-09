use std::fmt::Debug;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_infra::SafeRelativePath;

use crate::{
    catalog::LoaderRelease,
    error::{Error, Result},
    families::installer::InstallerMavenArtifactSpec,
    loader_version::LoaderVersionId,
    maven::MavenCoordinate,
    url::{Origin, OriginEndpoints},
};

pub trait MavenSpec: Clone + Debug + Send + Sync + 'static {
    type Origin: Origin;

    const SOURCE_NAME: &'static str;
    const MAVEN_ORIGIN: Self::Origin;
    const GROUP: &'static str;
    const ARTIFACT: &'static str;

    fn version(game_version: &MinecraftVersionId, loader_version: &LoaderVersionId) -> String {
        format!("{}-{}", game_version.as_str(), loader_version.as_str())
    }

    fn coordinate(version: &str) -> Result<MavenCoordinate> {
        let notation = format!("{}:{}:{version}", Self::GROUP, Self::ARTIFACT);
        MavenCoordinate::parse(&notation).ok_or_else(|| Error::InvalidNotation {
            label: Self::SOURCE_NAME.to_owned(),
            notation,
        })
    }

    fn metadata_path() -> Result<String> {
        let directory = MavenCoordinate::artifact_directory_path_for(Self::GROUP, Self::ARTIFACT)
            .ok_or_else(|| Error::InvalidNotation {
            label: Self::SOURCE_NAME.to_owned(),
            notation: format!("{}:{}", Self::GROUP, Self::ARTIFACT),
        })?;
        Ok(format!("/{directory}/maven-metadata.xml"))
    }

    fn installer_coordinate(version: &str) -> Result<MavenCoordinate> {
        let coordinate = Self::coordinate(version)?;
        coordinate
            .with_classifier("installer")
            .ok_or_else(|| Error::InvalidNotation {
                label: Self::SOURCE_NAME.to_owned(),
                notation: coordinate.to_string(),
            })
    }

    fn installer_artifact(
        endpoints: &OriginEndpoints<Self::Origin>,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec> {
        let version = Self::version(game_version, loader_version);
        let coordinate = Self::installer_coordinate(&version)?;
        let artifact_path = coordinate.artifact_path();
        Ok(InstallerMavenArtifactSpec {
            coordinate: coordinate.to_string(),
            download_url: endpoints.resolve(Self::MAVEN_ORIGIN, &artifact_path)?,
            relative_path: SafeRelativePath::parse(artifact_path)?,
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
