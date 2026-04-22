use std::path::PathBuf;

use anyhow::{Result, anyhow};
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::downloader::task::DownloadTask;

use crate::{
    families::version_json::{VersionJsonRootLayout, VersionJsonRootResource},
    maven::artifact_path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerArtifact {
    pub coordinate: String,
    pub url: String,
    pub path: PathBuf,
    pub expected_size: Option<u64>,
    pub sha1: Option<String>,
}

impl InstallerArtifact {
    pub fn download_task(&self) -> DownloadTask {
        DownloadTask::new(
            self.url.clone(),
            self.path.clone(),
            self.expected_size,
            self.sha1.clone(),
        )
    }
}

pub fn installer_coordinate_path(notation: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(artifact_path(
        notation,
        "installer artifact",
    )?))
}

pub fn installer_coordinate_file_name(notation: &str) -> Result<String> {
    let path = installer_coordinate_path(notation)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("installer coordinate has no file name: {notation}"))?;
    Ok(file_name)
}

pub fn build_installer_artifact<L>(
    game_storage: &Storage<L>,
    coordinate: String,
    url: String,
    library_relative_path: PathBuf,
) -> Result<InstallerArtifact>
where
    L: VersionJsonRootLayout,
{
    Ok(InstallerArtifact {
        coordinate,
        url,
        path: game_storage.try_get_extended_resource(VersionJsonRootResource::Libraries(Some(
            library_relative_path,
        )))?,
        expected_size: None,
        sha1: None,
    })
}
