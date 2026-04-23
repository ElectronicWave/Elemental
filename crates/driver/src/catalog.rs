use std::{collections::HashMap, future::Future};

use anyhow::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

#[async_trait]
pub trait Catalog {
    type Release: Release;

    async fn releases(&self) -> Result<HashMap<MinecraftVersionId, Vec<Self::Release>>>;
}

#[async_trait]
pub trait Release {
    async fn info(&self) -> ReleaseInfo;
}

pub struct ReleaseInfo {
    pub name: String,
    pub game_version: MinecraftVersionId,
    pub description: Option<String>,
}

pub fn single_game_release_info(
    name: String,
    game_version: MinecraftVersionId,
    description: Option<String>,
) -> ReleaseInfo {
    ReleaseInfo {
        name,
        game_version,
        description,
    }
}

pub fn push_single_game_release<R>(
    releases: &mut HashMap<MinecraftVersionId, Vec<R>>,
    game_version: MinecraftVersionId,
    release: R,
) {
    releases.entry(game_version).or_default().push(release);
}

pub fn collect_single_game_releases<Release, BuildRelease>(
    versions: Vec<String>,
    mut build_release: BuildRelease,
) -> HashMap<MinecraftVersionId, Vec<Release>>
where
    BuildRelease: FnMut(String) -> Option<(MinecraftVersionId, Release)>,
{
    let mut releases = HashMap::new();

    for version in versions {
        let Some((game_version, release)) = build_release(version) else {
            continue;
        };

        push_single_game_release(&mut releases, game_version, release);
    }

    releases
}

pub async fn collect_single_game_loader_releases<
    Loader,
    Release,
    LoadLoaderVersions,
    LoadLoaderVersionsFuture,
    BuildRelease,
>(
    game_versions: Vec<MinecraftVersionId>,
    mut load_loader_versions: LoadLoaderVersions,
    mut build_release: BuildRelease,
) -> Result<HashMap<MinecraftVersionId, Vec<Release>>>
where
    LoadLoaderVersions: FnMut(MinecraftVersionId) -> LoadLoaderVersionsFuture,
    LoadLoaderVersionsFuture: Future<Output = Result<Vec<Loader>>>,
    BuildRelease: FnMut(&MinecraftVersionId, Loader) -> Release,
{
    let mut releases = HashMap::new();

    for game_version in game_versions {
        let game_releases = load_loader_versions(game_version.clone())
            .await?
            .into_iter()
            .map(|loader| build_release(&game_version, loader))
            .collect::<Vec<Release>>();

        releases.insert(game_version, game_releases);
    }

    Ok(releases)
}
