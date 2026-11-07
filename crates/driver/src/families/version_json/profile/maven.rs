use std::collections::{HashMap, HashSet};

use elemental_schema::fabric::ProfileLibrary;
use elemental_schema::mojang::piston::{
    PistonMetaLibrariesDownloads, PistonMetaLibrariesDownloadsArtifact,
};

use crate::{
    error::Result,
    maven::{MavenNotation, MavenRepository},
};

pub(super) fn profile_library_downloads_from_maven(
    profile_library: &ProfileLibrary,
) -> Result<PistonMetaLibrariesDownloads> {
    let notation = MavenNotation::parse(profile_library.name.as_str(), "maven artifact")?;
    let repository = MavenRepository::parse(profile_library.url.as_str(), "maven repository")?;
    let artifact = repository.artifact_download(notation.artifact_path()?)?;
    let classifiers = profile_library
        .natives
        .as_ref()
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
