mod library;
mod manifest;

use std::collections::HashSet;

use elemental_infra::SafeRelativePath;
use elemental_schema::{
    fabric::{ProfileJson, ProfileLibrary},
    mojang::piston::{PistonMetaArguments, PistonMetaGenericArgument},
};

use crate::{error::Result, maven::MavenCoordinate};

use super::{
    manifest::{LiteLoaderManifestLibrary, LiteLoaderRepositoryStream, LiteLoaderSelectedBuild},
    source::LiteLoaderEndpoints,
};

use self::library::{profile_library_with_repository, rewritten_repository};
use self::manifest::normalize_manifest_library;

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

async fn build_loader_library(
    client: &reqwest::Client,
    endpoints: &LiteLoaderEndpoints,
    selected: &LiteLoaderSelectedBuild,
) -> Result<ProfileLibrary> {
    let notation = format!("com.mumfrey:liteloader:{}", selected.build.version);
    let parsed = MavenCoordinate::parse_labeled(notation.as_str(), "liteloader library")?;
    let repository = rewritten_repository(
        endpoints,
        selected.repository.url.as_str(),
        "liteloader repository",
    )?;
    let artifact = if selected.repository.stream == LiteLoaderRepositoryStream::Snapshot {
        repository
            .resolve_snapshot_artifact(client, &parsed)
            .await?
    } else {
        repository.artifact_download(SafeRelativePath::parse(format!(
            "{}/{}",
            parsed.safe_version_directory()?.as_str(),
            selected.build.file.as_str()
        ))?)?
    };

    Ok(profile_library_with_repository(
        notation,
        &repository,
        Some(artifact),
    ))
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
