mod library;
mod loader;
mod manifest;

use std::collections::HashSet;

use elemental_schema::{
    fabric::ProfileJson,
    mojang::piston::{PistonMetaArguments, PistonMetaGenericArgument},
};

use crate::error::Result;

use super::{
    manifest::{LiteLoaderManifestLibrary, LiteLoaderSelectedBuild},
    source::LiteLoaderEndpoints,
};

use self::loader::build_loader_library;
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
