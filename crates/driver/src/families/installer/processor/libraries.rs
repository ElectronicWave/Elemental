use std::collections::HashSet;

use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::SafeRelativePath;
use elemental_infra::downloader::{DownloadBatch, DownloadSession, DownloadTask};
use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    error::{Error, Result},
    families::{
        installer::libraries::installer_library_artifacts,
        version_json::{VersionJsonInstanceLayout, VersionJsonRootLayout, VersionJsonRootResource},
    },
};

pub async fn submit_installer_profile_libraries<L, VL, F>(
    session: &DownloadSession,
    instance: &Storage<VL, Storage<L>>,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
    artifact_url: F,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
    F: Fn(&str, &str) -> Result<String>,
{
    let tasks = install_profile_library_tasks(instance, install_profile, &artifact_url)?;

    if tasks.is_empty() {
        return Ok(());
    }

    session
        .submit(DownloadBatch::new(tasks).group("profile-libraries"))
        .await
        .map_err(|source| Error::DownloadInstallerProfileLibraries {
            family_name: family_name.to_owned(),
            source,
        })
}

fn install_profile_library_tasks<L, VL, F>(
    instance: &Storage<VL, Storage<L>>,
    install_profile: &ForgeInstallerProfile,
    artifact_url: &F,
) -> Result<Vec<DownloadTask>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
    F: Fn(&str, &str) -> Result<String>,
{
    let mut seen = HashSet::new();
    let mut tasks = Vec::new();

    for artifact in install_profile
        .libraries
        .iter()
        .flat_map(installer_library_artifacts)
    {
        let relative_path = SafeRelativePath::parse(artifact.path.as_str())?;
        let path = instance
            .parent
            .try_get_resource(VersionJsonRootResource::Libraries(Some(relative_path)))?;

        if !seen.insert(path.clone()) {
            continue;
        }

        let mut task = DownloadTask::new(
            artifact_url(artifact.url.as_str(), artifact.path.as_str())?,
            path,
        );
        if let Some(size) = artifact.size {
            task = task.size(size as u64);
        }
        if let Some(sha1) = &artifact.sha1 {
            task = task.sha1(sha1.clone());
        }
        tasks.push(task);
    }

    Ok(tasks)
}
