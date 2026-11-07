use std::collections::{HashMap, HashSet};

use crate::error::{Error, Result};

use super::{
    LiteLoaderLibraryResolution, LiteLoaderManifest, LiteLoaderManifestBuild,
    LiteLoaderManifestGameVersion, LiteLoaderManifestLibrary, LiteLoaderManifestRepository,
    LiteLoaderRelease, LiteLoaderSelectedBuild,
};

pub(crate) fn collect_releases(manifest: LiteLoaderManifest) -> Vec<LiteLoaderRelease> {
    let mut releases = Vec::new();

    for (game_version, game_entry) in manifest.versions {
        for selected in collect_builds_for_game(game_version.as_str(), &game_entry) {
            releases.push(LiteLoaderRelease {
                game_version: selected.game_version,
                loader_version: selected.build.version,
                stream: selected.build.stream,
            });
        }
    }

    releases.sort_by(|left, right| {
        left.game_version
            .cmp(&right.game_version)
            .then(left.loader_version.cmp(&right.loader_version))
    });

    releases
}

pub(crate) fn select_build(
    manifest: &LiteLoaderManifest,
    game_version: &str,
    loader_version: &str,
) -> Result<LiteLoaderSelectedBuild> {
    let game_entry = manifest.versions.get(game_version).ok_or_else(|| {
        Error::LiteLoaderMinecraftVersionNotFound {
            game_version: game_version.to_owned(),
        }
    })?;

    collect_builds_for_game(game_version, game_entry)
        .into_iter()
        .find(|build| build.build.version == loader_version)
        .ok_or_else(|| Error::LiteLoaderLoaderVersionNotFound {
            loader_version: loader_version.to_owned(),
            game_version: game_version.to_owned(),
        })
}

fn collect_builds_for_game(
    game_version: &str,
    game_entry: &LiteLoaderManifestGameVersion,
) -> Vec<LiteLoaderSelectedBuild> {
    let mut builds = Vec::new();
    let mut seen_versions = HashSet::new();

    collect_package_builds(
        &mut builds,
        &mut seen_versions,
        game_version,
        &game_entry.repo,
        Vec::new(),
        LiteLoaderLibraryResolution::Release,
        &game_entry.artefacts,
    );

    if let Some(snapshots) = &game_entry.snapshots {
        collect_package_builds(
            &mut builds,
            &mut seen_versions,
            game_version,
            &game_entry.repo,
            snapshots.libraries.clone(),
            LiteLoaderLibraryResolution::Snapshot,
            &snapshots.packages,
        );
    }

    builds.sort_by(|left, right| left.build.version.cmp(&right.build.version));

    builds
}

fn collect_package_builds(
    builds: &mut Vec<LiteLoaderSelectedBuild>,
    seen_versions: &mut HashSet<String>,
    game_version: &str,
    repository: &LiteLoaderManifestRepository,
    common_libraries: Vec<LiteLoaderManifestLibrary>,
    resolution: LiteLoaderLibraryResolution,
    packages: &HashMap<String, HashMap<String, LiteLoaderManifestBuild>>,
) {
    for (coordinates, entries) in packages {
        if coordinates != "com.mumfrey:liteloader" {
            continue;
        }

        for build in materialize_build_entries(entries, resolution) {
            if !build.version.is_empty() && seen_versions.insert(build.version.clone()) {
                builds.push(LiteLoaderSelectedBuild {
                    game_version: game_version.to_owned(),
                    repository: repository.clone(),
                    common_libraries: common_libraries
                        .iter()
                        .cloned()
                        .map(|library| library.with_resolution(resolution))
                        .collect(),
                    build,
                });
            }
        }
    }
}

fn materialize_build_entries(
    entries: &HashMap<String, LiteLoaderManifestBuild>,
    resolution: LiteLoaderLibraryResolution,
) -> Vec<LiteLoaderManifestBuild> {
    let mut builds = entries
        .iter()
        .filter(|(key, _)| key.as_str() != "latest")
        .map(|(_, build)| build.clone().with_library_resolution(resolution))
        .collect::<Vec<LiteLoaderManifestBuild>>();

    if builds.is_empty()
        && let Some(latest) = entries.get("latest")
    {
        builds.push(latest.clone().with_library_resolution(resolution));
    }

    builds
}
