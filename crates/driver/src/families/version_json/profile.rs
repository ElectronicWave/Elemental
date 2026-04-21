use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};
use reqwest::Url;

use elemental_schema::fabric::{
    ProfileJson, ProfileLibrary, ProfileLibraryArtifact, ProfileLibraryDownloads,
    ProfileLibraryExtract, ProfileLogging,
};

use crate::{
    families::version_json::{
        PistonMetaArguments, PistonMetaData, PistonMetaGenericArgument, PistonMetaLibraries,
        PistonMetaLibrariesDownloads, PistonMetaLibrariesDownloadsArtifact,
        PistonMetaLibrariesExtract, PistonMetaLogging, PistonMetaLoggingSide,
        PistonMetaLoggingSideFile,
    },
    launch_arguments::parse_argument_string,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryReplacementFamily {
    Asm,
    Lwjgl2,
}

pub trait ProfileMergeBehavior {
    fn profile_replacement_family(&self, _library_name: &str) -> Option<LibraryReplacementFamily> {
        None
    }

    fn replaced_base_family(&self, _library_name: &str) -> Option<LibraryReplacementFamily> {
        None
    }
}

pub struct PassthroughProfileBehavior;

pub const PASSTHROUGH_PROFILE_BEHAVIOR: PassthroughProfileBehavior = PassthroughProfileBehavior;

impl ProfileMergeBehavior for PassthroughProfileBehavior {}

pub fn merge_profile_with_behavior(
    behavior: &dyn ProfileMergeBehavior,
    base_metadata: PistonMetaData,
    profile: ProfileJson,
) -> Result<PistonMetaData> {
    let profile_logging = profile.logging.as_ref().map(profile_logging_to_piston);
    let merged_libraries = merge_libraries(
        base_metadata.libraries.clone(),
        &profile.libraries,
        behavior,
    )?;

    let arguments = merge_profile_arguments(
        base_metadata.arguments.clone(),
        base_metadata.minecraft_arguments.clone(),
        profile.arguments,
    )?;
    let minecraft_arguments = if arguments.is_some() {
        None
    } else {
        base_metadata.minecraft_arguments.clone()
    };

    Ok(PistonMetaData {
        arguments,
        minecraft_arguments,
        inherits_from: Some(profile.inherits_from),
        asset_index: base_metadata.asset_index,
        assets: profile.assets.unwrap_or(base_metadata.assets),
        compliance_level: base_metadata.compliance_level,
        downloads: base_metadata.downloads,
        id: profile.id,
        java_version: base_metadata.java_version,
        libraries: merged_libraries,
        logging: profile_logging.or(base_metadata.logging),
        main_class: profile.main_class,
        minimum_launcher_version: profile
            .minimum_launcher_version
            .unwrap_or(base_metadata.minimum_launcher_version),
        release_type: profile.release_type,
        time: profile.time,
        release_time: profile.release_time,
    })
}

pub fn metadata_has_replaced_library_conflicts<B>(behavior: &B, metadata: &PistonMetaData) -> bool
where
    B: ProfileMergeBehavior + ?Sized,
{
    let replacement_families = metadata
        .libraries
        .iter()
        .filter_map(|library| behavior.profile_replacement_family(library.name.as_str()))
        .collect::<HashSet<LibraryReplacementFamily>>();

    metadata.libraries.iter().any(|library| {
        let Some(family) = behavior.replaced_base_family(library.name.as_str()) else {
            return false;
        };

        replacement_families.contains(&family)
    })
}

fn merge_profile_arguments(
    base_arguments: Option<PistonMetaArguments>,
    base_minecraft_arguments: Option<String>,
    profile_arguments: Option<PistonMetaArguments>,
) -> Result<Option<PistonMetaArguments>> {
    let base_arguments = match (base_arguments, base_minecraft_arguments) {
        (Some(arguments), _) => Some(arguments),
        (None, Some(arguments)) => Some(PistonMetaArguments {
            game: parse_argument_string(arguments.as_str())
                .with_context(|| "parse base minecraftArguments failed".to_owned())?
                .into_iter()
                .map(PistonMetaGenericArgument::Plain)
                .collect(),
            jvm: Vec::new(),
        }),
        (None, None) => None,
    };

    match (base_arguments, profile_arguments) {
        (None, None) => Ok(None),
        (Some(base_arguments), None) => Ok(Some(base_arguments)),
        (None, Some(profile_arguments)) => Ok(Some(profile_arguments)),
        (Some(mut base_arguments), Some(profile_arguments)) => {
            base_arguments.game.extend(profile_arguments.game);
            base_arguments.jvm.extend(profile_arguments.jvm);
            Ok(Some(base_arguments))
        }
    }
}

fn merge_libraries(
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

fn profile_library_downloads_from_maven(
    profile_library: &ProfileLibrary,
) -> Result<PistonMetaLibrariesDownloads> {
    let artifact_path = maven_artifact_path(profile_library.name.as_str())?;
    let artifact_url = maven_artifact_url(profile_library.url.as_str(), artifact_path.as_str())?;
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
                    let classifier_notation = maven_classifier_notation(
                        profile_library.name.as_str(),
                        classifier.as_str(),
                    );
                    let path = maven_artifact_path(classifier_notation.as_str())?;
                    let url = maven_artifact_url(profile_library.url.as_str(), path.as_str())?;

                    Ok((
                        classifier,
                        PistonMetaLibrariesDownloadsArtifact {
                            sha1: None,
                            size: None,
                            url,
                            path,
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
            url: artifact_url,
            path: artifact_path,
        }),
        classifiers,
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

fn maven_artifact_path(notation: &str) -> Result<String> {
    let (coordinates, extension) = notation.split_once('@').unwrap_or((notation, "jar"));
    let segments = coordinates.split(':').collect::<Vec<&str>>();

    let (group, artifact, version, classifier) = match segments.as_slice() {
        [group, artifact, version] => (*group, *artifact, *version, None),
        [group, artifact, version, classifier] => (*group, *artifact, *version, Some(*classifier)),
        _ => bail!("invalid maven artifact notation: {notation}"),
    };

    let group_path = group.replace('.', "/");
    let file_name = match classifier {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.{extension}"),
        None => format!("{artifact}-{version}.{extension}"),
    };

    Ok(format!("{group_path}/{artifact}/{version}/{file_name}"))
}

fn maven_artifact_url(base_url: &str, artifact_path: &str) -> Result<String> {
    let normalized_base_url = if base_url.ends_with('/') {
        base_url.to_owned()
    } else {
        format!("{base_url}/")
    };
    let base = Url::parse(normalized_base_url.as_str())
        .with_context(|| format!("parse library base url failed: {base_url}"))?;

    base.join(artifact_path)
        .with_context(|| format!("join library artifact url failed: {base_url} + {artifact_path}"))
        .map(|url| url.to_string())
}

fn maven_classifier_notation(notation: &str, classifier: &str) -> String {
    let (coordinates, extension) = notation.split_once('@').unwrap_or((notation, "jar"));
    format!("{coordinates}:{classifier}@{extension}")
}

fn profile_logging_to_piston(logging: &ProfileLogging) -> PistonMetaLogging {
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
