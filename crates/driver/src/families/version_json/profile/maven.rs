use std::collections::{HashMap, HashSet};

use elemental_schema::mojang::piston::{
    PistonMetaLibrariesDownloads, PistonMetaLibrariesDownloadsArtifact,
};

use crate::{
    error::Result,
    maven::{MavenNotation, MavenRepository},
};

pub(super) fn profile_library_downloads_from_maven(
    name: &str,
    url: &str,
    natives: Option<&HashMap<String, String>>,
) -> Result<PistonMetaLibrariesDownloads> {
    let notation = MavenNotation::parse(name, "maven artifact")?;
    let repository = MavenRepository::parse(url, "maven repository")?;
    let artifact = repository.artifact_download(notation.artifact_path()?)?;
    let classifiers = natives
        .map(|natives| {
            natives
                .values()
                .cloned()
                .collect::<HashSet<String>>()
                .into_iter()
                .map(|classifier| {
                    let classifier_notation = notation.with_classifier(classifier.as_str())?;
                    let artifact =
                        repository.artifact_download(classifier_notation.artifact_path()?)?;

                    Ok((
                        classifier,
                        PistonMetaLibrariesDownloadsArtifact {
                            sha1: None,
                            size: None,
                            url: artifact.url,
                            path: artifact.path.as_str().to_owned(),
                        },
                    ))
                })
                .collect::<Result<HashMap<String, PistonMetaLibrariesDownloadsArtifact>>>()
        })
        .transpose()?;

    Ok(PistonMetaLibrariesDownloads {
        artifact: Some(PistonMetaLibrariesDownloadsArtifact {
            sha1: None,
            size: None,
            url: artifact.url,
            path: artifact.path.as_str().to_owned(),
        }),
        classifiers,
    })
}
