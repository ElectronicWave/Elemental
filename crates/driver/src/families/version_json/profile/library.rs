use std::collections::{HashMap, HashSet};

use elemental_schema::fabric::{
    ProfileLibrary, ProfileLibraryArtifact, ProfileLibraryDownloads, ProfileLibraryExtract,
    ProfileLogging,
};
use elemental_schema::mojang::piston::{
    PistonMetaLibraries, PistonMetaLibrariesDownloads, PistonMetaLibrariesDownloadsArtifact,
    PistonMetaLibrariesExtract, PistonMetaLogging, PistonMetaLoggingSide,
    PistonMetaLoggingSideFile,
};

use crate::error::Result;

use super::super::LibraryReplacementFamily;
use super::super::ProfileMergeBehavior;
use super::maven::profile_library_downloads_from_maven;

pub(super) fn merge_libraries(
    base_libraries: Vec<PistonMetaLibraries>,
    profile_libraries: &[ProfileLibrary],
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

fn profile_library_to_piston(profile_library: &ProfileLibrary) -> Result<PistonMetaLibraries> {
    if let Some(downloads) = &profile_library.downloads {
        return Ok(PistonMetaLibraries {
            downloads: profile_library_downloads_to_piston(downloads),
            name: profile_library.name.clone(),
            natives: profile_library.natives.clone(),
            rules: None,
            extract: profile_library
                .extract
                .as_ref()
                .map(profile_library_extract_to_piston),
        });
    }

    let downloads = profile_library_downloads_from_maven(profile_library)?;

    Ok(PistonMetaLibraries {
        downloads,
        name: profile_library.name.clone(),
        natives: profile_library.natives.clone(),
        rules: None,
        extract: profile_library
            .extract
            .as_ref()
            .map(profile_library_extract_to_piston),
    })
}

fn profile_library_downloads_to_piston(
    downloads: &ProfileLibraryDownloads,
) -> PistonMetaLibrariesDownloads {
    PistonMetaLibrariesDownloads {
        artifact: downloads
            .artifact
            .as_ref()
            .map(profile_library_artifact_to_piston),
        classifiers: downloads.classifiers.as_ref().map(|classifiers| {
            classifiers
                .iter()
                .map(|(key, artifact)| (key.clone(), profile_library_artifact_to_piston(artifact)))
                .collect::<HashMap<String, PistonMetaLibrariesDownloadsArtifact>>()
        }),
    }
}

fn profile_library_artifact_to_piston(
    artifact: &ProfileLibraryArtifact,
) -> PistonMetaLibrariesDownloadsArtifact {
    PistonMetaLibrariesDownloadsArtifact {
        sha1: artifact.sha1.clone(),
        size: artifact.size,
        url: artifact.url.clone(),
        path: artifact.path.clone(),
    }
}

fn profile_library_extract_to_piston(
    extract: &ProfileLibraryExtract,
) -> PistonMetaLibrariesExtract {
    PistonMetaLibrariesExtract {
        exclude: extract.exclude.clone(),
    }
}

pub(super) fn profile_logging_to_piston(logging: &ProfileLogging) -> PistonMetaLogging {
    PistonMetaLogging {
        client: Some(PistonMetaLoggingSide {
            argument: logging.client.argument.clone(),
            file: PistonMetaLoggingSideFile {
                id: logging.client.file.id.clone(),
                sha1: logging.client.file.sha1.clone(),
                size: logging.client.file.size,
                url: logging.client.file.url.clone(),
            },
            logging_type: logging.client.logging_type.clone(),
        }),
    }
}
