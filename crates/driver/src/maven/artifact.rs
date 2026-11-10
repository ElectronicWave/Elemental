use std::collections::HashMap;

use elemental_schema::mojang::piston::{
    PistonMetaLibrariesDownloads, PistonMetaLibrariesDownloadsArtifact,
};

use crate::error::{Error, Result};

use super::{MavenCoordinate, MavenRepository};

pub fn maven_library_downloads(
    coordinate: &MavenCoordinate,
    repository_url: &str,
    sha1: Option<String>,
    size: Option<usize>,
    natives: Option<&HashMap<String, String>>,
    arch_bits: &str,
) -> Result<PistonMetaLibrariesDownloads> {
    let repository = MavenRepository::parse(repository_url, "maven repository")?;

    let artifact = maven_artifact(&repository, coordinate, sha1, size)?;
    let classifiers = natives
        .map(|natives| {
            natives
                .values()
                .map(|classifier| {
                    let classifier = classifier.replace("${arch}", arch_bits);
                    let classifier_coordinate = coordinate
                        .with_classifier(classifier.as_str())
                        .ok_or_else(|| Error::InvalidNotation {
                            label: "maven classifier".to_owned(),
                            notation: classifier.clone(),
                        })?;
                    let artifact = maven_artifact(&repository, &classifier_coordinate, None, None)?;

                    Ok((classifier, artifact))
                })
                .collect::<Result<HashMap<_, _>>>()
        })
        .transpose()?;

    Ok(PistonMetaLibrariesDownloads {
        artifact: Some(artifact),
        classifiers,
    })
}

fn maven_artifact(
    repository: &MavenRepository,
    coordinate: &MavenCoordinate,
    sha1: Option<String>,
    size: Option<usize>,
) -> Result<PistonMetaLibrariesDownloadsArtifact> {
    let download = repository.artifact_download(coordinate.safe_artifact_path()?)?;

    Ok(PistonMetaLibrariesDownloadsArtifact {
        sha1,
        size,
        url: download.url,
        path: download.path.as_str().to_owned(),
    })
}
