use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use reqwest::Url;

use elemental_schema::fabric::{ProfileJson, ProfileLibrary, ProfileLogging};

use crate::drivers::{
    fabric::source::FabricEndpoints,
    vanilla::source::VanillaEndpoints,
    version_json::{
        LaunchedVersionJsonInstance, PistonMetaData, PistonMetaLibraries,
        PistonMetaLibrariesDownloads, PistonMetaLibrariesDownloadsArtifact, PistonMetaLogging,
        PistonMetaLoggingSide, PistonMetaLoggingSideFile, PreparedVersionJsonInstance,
        ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata, VersionJsonInstallStatus,
        VersionJsonRemoteResolver,
    },
};

#[derive(Debug, Clone)]
pub struct FabricRemoteResolver {
    vanilla_endpoints: VanillaEndpoints,
    fabric_endpoints: FabricEndpoints,
}

pub type ResolvedFabricMetadata = ResolvedVersionJsonMetadata<FabricRemoteResolver>;
pub type FabricInstallStatus = VersionJsonInstallStatus;
pub type ResolvedFabricVersion<L, VL> = ResolvedVersionJsonInstance<FabricRemoteResolver, L, VL>;
pub type PreparedFabricVersion<L, VL> = PreparedVersionJsonInstance<FabricRemoteResolver, L, VL>;
pub type LaunchedFabricVersion<L, VL> = LaunchedVersionJsonInstance<FabricRemoteResolver, L, VL>;

impl FabricRemoteResolver {
    pub fn new(vanilla_endpoints: VanillaEndpoints, fabric_endpoints: FabricEndpoints) -> Self {
        Self {
            vanilla_endpoints,
            fabric_endpoints,
        }
    }
}

impl VersionJsonRemoteResolver for FabricRemoteResolver {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        if let Ok(rewritten) = self.vanilla_endpoints.rewrite_upstream(raw_url) {
            return Ok(rewritten);
        }

        self.fabric_endpoints
            .rewrite_upstream(raw_url)
            .with_context(|| format!("rewrite fabric upstream url failed for '{raw_url}'"))
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }
}

pub fn merge_fabric_profile(
    base_metadata: PistonMetaData,
    profile: ProfileJson,
) -> Result<PistonMetaData> {
    let profile_logging = profile.logging.as_ref().map(profile_logging_to_piston);
    let merged_libraries = merge_libraries(base_metadata.libraries.clone(), &profile.libraries)?;

    let arguments = merge_profile_arguments(
        base_metadata.arguments.clone(),
        base_metadata.minecraft_arguments.clone(),
        profile.arguments,
    );
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
        minimum_launcher_version: profile.minimum_launcher_version,
        release_type: profile.release_type,
        time: profile.time,
        release_time: profile.release_time,
    })
}

fn merge_profile_arguments(
    base_arguments: Option<crate::drivers::version_json::PistonMetaArguments>,
    base_minecraft_arguments: Option<String>,
    profile_arguments: Option<crate::drivers::version_json::PistonMetaArguments>,
) -> Option<crate::drivers::version_json::PistonMetaArguments> {
    let base_arguments = base_arguments.or_else(|| {
        base_minecraft_arguments.map(|arguments| {
            crate::drivers::version_json::PistonMetaArguments {
                game: arguments
                    .split_whitespace()
                    .map(|argument| {
                        crate::drivers::version_json::PistonMetaGenericArgument::Plain(
                            argument.to_owned(),
                        )
                    })
                    .collect(),
                jvm: Vec::new(),
            }
        })
    });

    match (base_arguments, profile_arguments) {
        (None, None) => None,
        (Some(base_arguments), None) => Some(base_arguments),
        (None, Some(profile_arguments)) => Some(profile_arguments),
        (Some(mut base_arguments), Some(profile_arguments)) => {
            base_arguments.game.extend(profile_arguments.game);
            base_arguments.jvm.extend(profile_arguments.jvm);
            Some(base_arguments)
        }
    }
}

fn merge_libraries(
    base_libraries: Vec<PistonMetaLibraries>,
    profile_libraries: &[ProfileLibrary],
) -> Result<Vec<PistonMetaLibraries>> {
    let mut merged = Vec::new();
    let mut seen = HashSet::new();

    for library in base_libraries {
        if seen.insert(library.name.clone()) {
            merged.push(library);
        }
    }

    for profile_library in profile_libraries {
        let library = profile_library_to_piston(profile_library)?;
        if seen.insert(library.name.clone()) {
            merged.push(library);
        }
    }

    Ok(merged)
}

fn profile_library_to_piston(profile_library: &ProfileLibrary) -> Result<PistonMetaLibraries> {
    let artifact_path = maven_artifact_path(profile_library.name.as_str())?;
    let artifact_url = maven_artifact_url(profile_library.url.as_str(), artifact_path.as_str())?;

    Ok(PistonMetaLibraries {
        downloads: PistonMetaLibrariesDownloads {
            artifact: PistonMetaLibrariesDownloadsArtifact {
                sha1: None,
                size: None,
                url: artifact_url,
                path: artifact_path,
            },
            classifiers: None,
        },
        name: profile_library.name.clone(),
        natives: None,
        rules: None,
        extract: None,
    })
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
    let mut url = Url::parse(base_url)
        .with_context(|| format!("parse library base url failed: {base_url}"))?;
    {
        let mut path_segments = url
            .path_segments_mut()
            .map_err(|_| anyhow::anyhow!("library base url cannot be used as a path base"))?;
        for segment in artifact_path.split('/') {
            path_segments.push(segment);
        }
    }

    Ok(url.to_string())
}

fn profile_logging_to_piston(logging: &ProfileLogging) -> PistonMetaLogging {
    PistonMetaLogging {
        client: PistonMetaLoggingSide {
            argument: logging.client.argument.clone(),
            file: PistonMetaLoggingSideFile {
                id: logging.client.file.id.clone(),
                sha1: logging.client.file.sha1.clone(),
                size: logging.client.file.size,
                url: logging.client.file.url.clone(),
            },
            logging_type: logging.client.logging_type.clone(),
        },
    }
}
