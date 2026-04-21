use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};
use elemental_infra::downloader::task::DownloadTask;

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
    Ok(PathBuf::from(maven_artifact_path(notation)?))
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

fn maven_artifact_path(notation: &str) -> Result<String> {
    let (coordinates, extension) = notation.split_once('@').unwrap_or((notation, "jar"));
    let segments = coordinates.split(':').collect::<Vec<&str>>();

    let (group, artifact, version, classifier) = match segments.as_slice() {
        [group, artifact, version] => (*group, *artifact, *version, None),
        [group, artifact, version, classifier] => (*group, *artifact, *version, Some(*classifier)),
        _ => bail!("invalid installer artifact notation: {notation}"),
    };

    let group_path = group.replace('.', "/");
    let file_name = match classifier {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.{extension}"),
        None => format!("{artifact}-{version}.{extension}"),
    };

    Ok(format!("{group_path}/{artifact}/{version}/{file_name}"))
}
