use std::collections::HashSet;

use elemental_core::storage::layout::Layoutable;
use elemental_infra::SafeRelativePath;
use elemental_infra::downloader::{DownloadBatch, DownloadSession, DownloadTask};
use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    error::{Error, Result},
    families::{
        installer::libraries::installer_library_artifacts,
        version_json::{GameStorage, VersionJsonRootResource},
    },
};

pub async fn submit_installer_profile_libraries(
    session: &DownloadSession,
    game_storage: &GameStorage,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
    artifact_url: &(dyn Fn(&str, &str) -> Result<String> + Send + Sync),
) -> Result<()> {
    let tasks = install_profile_library_tasks(game_storage, install_profile, artifact_url)?;

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

fn install_profile_library_tasks(
    game_storage: &GameStorage,
    install_profile: &ForgeInstallerProfile,
    artifact_url: &dyn Fn(&str, &str) -> Result<String>,
) -> Result<Vec<DownloadTask>> {
    let mut seen = HashSet::new();
    let mut tasks = Vec::new();

    for library in &install_profile.libraries {
        for artifact in installer_library_artifacts(library)? {
            let relative_path = SafeRelativePath::parse(artifact.path.as_str())?;
            let path = game_storage
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
    }

    Ok(tasks)
}
