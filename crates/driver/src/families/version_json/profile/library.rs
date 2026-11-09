use std::collections::{HashMap, HashSet};

use elemental_schema::fabric::{
    ProfileLibrary, ProfileLibraryArtifact, ProfileLibraryDownloads, ProfileLogging,
};
use elemental_schema::mojang::piston::{
    PistonMetaLibraries, PistonMetaLibrariesDownloads, PistonMetaLibrariesDownloadsArtifact,
    PistonMetaLibrariesExtract, PistonMetaLibrariesSource, PistonMetaLogging,
    PistonMetaLoggingSide, PistonMetaLoggingSideFile,
};

use crate::error::Result;

use super::super::LibraryReplacementFamily;
use super::super::ProfileMergeBehavior;
use super::maven::profile_library_downloads_from_maven;

pub(super) fn merge_libraries(
    base_libraries: Vec<PistonMetaLibraries>,
    profile_libraries: Vec<ProfileLibrary>,
    behavior: &dyn ProfileMergeBehavior,
) -> Result<Vec<PistonMetaLibraries>> {
    let replacement_families = profile_libraries
        .iter()
        .filter_map(|library| behavior.profile_replacement_family(library.name.as_str()))
        .collect::<HashSet<LibraryReplacementFamily>>();
    let filtered_base_libraries = base_libraries
        .into_iter()
        .filter(|library| {
            let Some(family) = behavior.replaced_base_family(library.name.as_str()) else {
                return true;
            };

            !replacement_families.contains(&family)
        })
        .collect::<Vec<PistonMetaLibraries>>();
    let mut seen = filtered_base_libraries
        .iter()
        .map(|library| library.name.clone())
        .collect::<HashSet<String>>();
    let mut merged = filtered_base_libraries;

    for profile_library in profile_libraries {
        let library = profile_library_to_piston(profile_library)?;
        if seen.insert(library.name.clone()) {
            merged.push(library);
        }
    }

    Ok(merged)
}

fn profile_library_to_piston(profile_library: ProfileLibrary) -> Result<PistonMetaLibraries> {
    let ProfileLibrary {
        name,
        url,
        downloads,
        natives,
        extract,
    } = profile_library;
    let downloads = match downloads {
        Some(downloads) => profile_library_downloads_to_piston(downloads),
        None => {
            profile_library_downloads_from_maven(name.as_str(), url.as_str(), natives.as_ref())?
        }
    };

    Ok(PistonMetaLibraries {
        name,
        source: PistonMetaLibrariesSource::Downloads {
            downloads,
            extra: serde_json::Map::new(),
        },
        natives,
        rules: None,
        extract: extract.map(|extract| PistonMetaLibrariesExtract {
            exclude: extract.exclude,
        }),
    })
}

fn profile_library_downloads_to_piston(
    downloads: ProfileLibraryDownloads,
) -> PistonMetaLibrariesDownloads {
    PistonMetaLibrariesDownloads {
        artifact: downloads.artifact.map(profile_library_artifact_to_piston),
        classifiers: downloads.classifiers.map(|classifiers| {
            classifiers
                .into_iter()
                .map(|(key, artifact)| (key, profile_library_artifact_to_piston(artifact)))
                .collect::<HashMap<String, PistonMetaLibrariesDownloadsArtifact>>()
        }),
    }
}

fn profile_library_artifact_to_piston(
    artifact: ProfileLibraryArtifact,
) -> PistonMetaLibrariesDownloadsArtifact {
    PistonMetaLibrariesDownloadsArtifact {
        sha1: artifact.sha1,
        size: artifact.size,
        url: artifact.url,
        path: artifact.path,
    }
}

pub(super) fn profile_logging_to_piston(logging: ProfileLogging) -> PistonMetaLogging {
    PistonMetaLogging {
        client: Some(PistonMetaLoggingSide {
            argument: logging.client.argument,
            file: PistonMetaLoggingSideFile {
                id: logging.client.file.id,
                sha1: logging.client.file.sha1,
                size: logging.client.file.size,
                url: logging.client.file.url,
            },
            logging_type: logging.client.logging_type,
        }),
    }
}
