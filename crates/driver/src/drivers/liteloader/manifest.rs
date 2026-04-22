use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteLoaderRelease {
    pub game_version: String,
    pub loader_version: String,
    pub stream: String,
}

#[derive(Debug, Clone)]
pub(super) struct LiteLoaderSelectedBuild {
    pub(super) game_version: String,
    pub(super) repository: LiteLoaderManifestRepository,
    pub(super) common_libraries: Vec<LiteLoaderManifestLibrary>,
    pub(super) build: LiteLoaderManifestBuild,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifest {
    pub(super) versions: HashMap<String, LiteLoaderManifestGameVersion>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifestGameVersion {
    pub(super) repo: LiteLoaderManifestRepository,
    #[serde(default)]
    pub(super) artefacts: HashMap<String, HashMap<String, LiteLoaderManifestBuild>>,
    pub(super) snapshots: Option<LiteLoaderManifestSnapshots>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifestRepository {
    #[serde(rename = "stream")]
    _stream: String,
    #[serde(rename = "type")]
    _repository_type: String,
    pub(super) url: String,
    #[serde(rename = "classifier")]
    _classifier: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifestSnapshots {
    #[serde(default)]
    pub(super) libraries: Vec<LiteLoaderManifestLibrary>,
    #[serde(flatten)]
    pub(super) packages: HashMap<String, HashMap<String, LiteLoaderManifestBuild>>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifestBuild {
    #[serde(rename = "tweakClass")]
    pub(super) tweak_class: String,
    #[serde(default)]
    pub(super) libraries: Vec<LiteLoaderManifestLibrary>,
    pub(super) stream: String,
    pub(super) file: String,
    pub(super) version: String,
    #[serde(rename = "build")]
    _build: Option<String>,
    #[serde(rename = "md5")]
    _md5: Option<String>,
    pub(super) timestamp: Option<String>,
    #[serde(rename = "lastSuccessfulBuild")]
    _last_successful_build: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifestLibrary {
    pub(super) name: String,
    #[serde(default)]
    pub(super) url: String,
}

pub(super) fn collect_releases(manifest: LiteLoaderManifest) -> Vec<LiteLoaderRelease> {
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

pub(super) fn select_build(
    manifest: &LiteLoaderManifest,
    game_version: &str,
    loader_version: &str,
) -> Result<LiteLoaderSelectedBuild> {
    let game_entry = manifest
        .versions
        .get(game_version)
        .with_context(|| format!("can't find LiteLoader Minecraft version '{game_version}'"))?;

    collect_builds_for_game(game_version, game_entry)
        .into_iter()
        .find(|build| build.build.version == loader_version)
        .with_context(|| {
            format!(
                "can't find LiteLoader loader version '{loader_version}' for Minecraft '{game_version}'"
            )
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
        &game_entry.artefacts,
    );

    if let Some(snapshots) = &game_entry.snapshots {
        collect_package_builds(
            &mut builds,
            &mut seen_versions,
            game_version,
            &game_entry.repo,
            snapshots.libraries.clone(),
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
    packages: &HashMap<String, HashMap<String, LiteLoaderManifestBuild>>,
) {
    for (coordinates, entries) in packages {
        if coordinates != "com.mumfrey:liteloader" {
            continue;
        }

        for build in materialize_build_entries(entries) {
            if !build.version.is_empty() && seen_versions.insert(build.version.clone()) {
                builds.push(LiteLoaderSelectedBuild {
                    game_version: game_version.to_owned(),
                    repository: repository.clone(),
                    common_libraries: common_libraries.clone(),
                    build,
                });
            }
        }
    }
}

fn materialize_build_entries(
    entries: &HashMap<String, LiteLoaderManifestBuild>,
) -> Vec<LiteLoaderManifestBuild> {
    let mut builds = entries
        .iter()
        .filter(|(key, _)| key.as_str() != "latest")
        .map(|(_, build)| build.clone())
        .collect::<Vec<LiteLoaderManifestBuild>>();

    if builds.is_empty()
        && let Some(latest) = entries.get("latest")
    {
        builds.push(latest.clone());
    }

    builds
}
