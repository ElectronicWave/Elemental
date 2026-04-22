use std::collections::HashSet;

use anyhow::{Result, bail};
use elemental_schema::{
    fabric::{ProfileJson, ProfileLibrary, ProfileLibraryArtifact, ProfileLibraryDownloads},
    mojang::piston::{PistonMetaArguments, PistonMetaGenericArgument},
};

use super::{
    manifest::{LiteLoaderManifestLibrary, LiteLoaderSelectedBuild},
    maven::{
        ResolvedArtifactDownload, ensure_trailing_slash, is_snapshot_version,
        is_snapshot_version_from_notation, join_artifact_url, parse_maven_notation,
        resolve_snapshot_artifact,
    },
    source::LiteLoaderEndpoints,
};

pub(super) async fn build_profile_json(
    client: &reqwest::Client,
    endpoints: &LiteLoaderEndpoints,
    selected: &LiteLoaderSelectedBuild,
) -> Result<ProfileJson> {
    let mut libraries = Vec::new();
    libraries.push(build_loader_library(client, endpoints, selected).await?);

    for library in collect_manifest_libraries(selected) {
        libraries.push(normalize_manifest_library(client, endpoints, library).await?);
    }

    let timestamp = selected
        .build
        .timestamp
        .clone()
        .unwrap_or_else(|| "0".to_owned());

    Ok(ProfileJson {
        id: format!(
            "{}-liteloader-{}",
            selected.game_version, selected.build.version
        ),
        inherits_from: selected.game_version.clone(),
        arguments: Some(PistonMetaArguments {
            game: vec![
                PistonMetaGenericArgument::Plain("--tweakClass".to_owned()),
                PistonMetaGenericArgument::Plain(selected.build.tweak_class.clone()),
            ],
            jvm: Vec::new(),
        }),
        assets: None,
        libraries,
        logging: None,
        main_class: "net.minecraft.launchwrapper.Launch".to_owned(),
        minimum_launcher_version: None,
        release_type: selected.build.stream.to_ascii_lowercase(),
        time: timestamp.clone(),
        release_time: timestamp,
    })
}

fn collect_manifest_libraries(
    selected: &LiteLoaderSelectedBuild,
) -> Vec<LiteLoaderManifestLibrary> {
    let mut seen = HashSet::new();
    let mut libraries = Vec::new();

    for library in selected
        .common_libraries
        .iter()
        .chain(selected.build.libraries.iter())
    {
        if seen.insert(library.name.clone()) {
            libraries.push(library.clone());
        }
    }

    libraries
}

async fn build_loader_library(
    client: &reqwest::Client,
    endpoints: &LiteLoaderEndpoints,
    selected: &LiteLoaderSelectedBuild,
) -> Result<ProfileLibrary> {
    let notation = format!("com.mumfrey:liteloader:{}", selected.build.version);
    let parsed = parse_maven_notation(notation.as_str(), "liteloader library")?;
    let repository_base_url = ensure_trailing_slash(
        endpoints
            .rewrite_upstream(selected.repository.url.as_str())?
            .as_str(),
    );
    let artifact = if is_snapshot_version(selected.build.version.as_str()) {
        resolve_snapshot_artifact(client, repository_base_url.as_str(), &parsed).await?
    } else {
        let path = format!(
            "{}/{}",
            parsed.version_directory_path(),
            selected.build.file.as_str()
        );
        let url = join_artifact_url(repository_base_url.as_str(), path.as_str())?;
        ResolvedArtifactDownload { path, url }
    };

    Ok(ProfileLibrary {
        name: notation,
        url: repository_base_url.clone(),
        downloads: Some(ProfileLibraryDownloads {
            artifact: Some(ProfileLibraryArtifact {
                path: artifact.path,
                url: artifact.url,
                size: None,
                sha1: None,
            }),
            classifiers: None,
        }),
        natives: None,
        extract: None,
    })
}

async fn normalize_manifest_library(
    client: &reqwest::Client,
    endpoints: &LiteLoaderEndpoints,
    library: LiteLoaderManifestLibrary,
) -> Result<ProfileLibrary> {
    if library.name.starts_with("net.minecraft:") {
        return Ok(ProfileLibrary {
            name: library.name,
            url: ensure_trailing_slash(endpoints.mojang_libraries_url()?.as_str()),
            downloads: None,
            natives: None,
            extract: None,
        });
    }

    if library.name.starts_with("org.ow2.asm:") {
        return Ok(ProfileLibrary {
            name: library.name,
            url: ensure_trailing_slash(endpoints.maven_central_url()?.as_str()),
            downloads: None,
            natives: None,
            extract: None,
        });
    }

    if is_snapshot_version_from_notation(library.name.as_str()) {
        let repository_url = if library.url.is_empty() {
            default_snapshot_repository(endpoints, library.name.as_str())?
        } else {
            ensure_trailing_slash(endpoints.rewrite_upstream(library.url.as_str())?.as_str())
        };
        let parsed = parse_maven_notation(library.name.as_str(), "liteloader snapshot library")?;
        let artifact = resolve_snapshot_artifact(client, repository_url.as_str(), &parsed).await?;

        return Ok(ProfileLibrary {
            name: library.name,
            url: repository_url,
            downloads: Some(ProfileLibraryDownloads {
                artifact: Some(ProfileLibraryArtifact {
                    path: artifact.path,
                    url: artifact.url,
                    size: None,
                    sha1: None,
                }),
                classifiers: None,
            }),
            natives: None,
            extract: None,
        });
    }

    if !library.url.is_empty() {
        return Ok(ProfileLibrary {
            name: library.name,
            url: ensure_trailing_slash(endpoints.rewrite_upstream(library.url.as_str())?.as_str()),
            downloads: None,
            natives: None,
            extract: None,
        });
    }

    bail!(
        "LiteLoader library '{}' does not declare a supported repository",
        library.name
    )
}

fn default_snapshot_repository(
    endpoints: &LiteLoaderEndpoints,
    library_name: &str,
) -> Result<String> {
    if library_name.starts_with("org.spongepowered:") {
        return endpoints
            .sponge_maven_url()
            .map(|url| ensure_trailing_slash(url.as_str()));
    }

    bail!(
        "LiteLoader snapshot library '{}' does not declare a repository",
        library_name
    )
}
