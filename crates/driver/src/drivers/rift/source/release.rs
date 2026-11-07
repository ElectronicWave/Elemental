use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiftRelease {
    pub tag_name: String,
    pub loader_version: String,
    pub asset_name: String,
    pub asset_size: Option<usize>,
    pub published_at: Option<String>,
    pub prerelease: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct GitHubRelease {
    pub(super) tag_name: String,
    pub(super) draft: bool,
    pub(super) prerelease: bool,
    pub(super) published_at: Option<String>,
    pub(super) assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GitHubReleaseAsset {
    pub(super) name: String,
    pub(super) size: usize,
}

pub(super) fn build_rift_release(release: GitHubRelease) -> Result<RiftRelease> {
    let loader_version = normalize_loader_version(release.tag_name.as_str())?;
    let expected_asset_name = format!("Rift-{loader_version}.jar");
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == expected_asset_name)
        .ok_or_else(|| Error::RiftReleaseAssetNotFound {
            asset_name: expected_asset_name,
        })?;

    Ok(RiftRelease {
        tag_name: release.tag_name,
        loader_version,
        asset_name: asset.name,
        asset_size: Some(asset.size),
        published_at: release.published_at,
        prerelease: release.prerelease,
    })
}

pub(super) fn release_for_loader_version(loader_version: &str) -> RiftRelease {
    RiftRelease {
        tag_name: format!("v{loader_version}"),
        loader_version: loader_version.to_owned(),
        asset_name: format!("Rift-{loader_version}.jar"),
        asset_size: None,
        published_at: None,
        prerelease: false,
    }
}

fn normalize_loader_version(tag_name: &str) -> Result<String> {
    let loader_version = tag_name.trim_start_matches('v').trim();
    if loader_version.is_empty() {
        return Err(Error::RiftReleaseTagMissingLoaderVersion {
            tag_name: tag_name.to_owned(),
        });
    }

    Ok(loader_version.to_owned())
}
