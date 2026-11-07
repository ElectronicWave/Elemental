use std::path::Path;

use elemental_infra::downloader::{DownloadBatch, DownloadSession};
use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    error::{Error, Result},
    families::installer::{InstallerArchive, InstallerArtifact},
};

use super::{InstallerPersistedState, InstallerStatePaths, state::read_embedded_version};

pub async fn prepare_installer_state<V>(
    session: &DownloadSession,
    installer_artifact: &InstallerArtifact,
    instance_root: &Path,
    libraries_root: &Path,
    family_name: &str,
    validate_profile: V,
) -> Result<InstallerPersistedState>
where
    V: Fn(&ForgeInstallerProfile) -> Result<()>,
{
    session
        .submit(DownloadBatch::new(vec![installer_artifact.download_task()]).group("installer"))
        .await
        .map_err(|source| Error::DownloadInstaller {
            family_name: family_name.to_owned(),
            source,
        })?;
    session
        .wait_idle()
        .await
        .map_err(|source| Error::DownloadInstaller {
            family_name: family_name.to_owned(),
            source,
        })?;
    session
        .success()
        .await
        .map_err(|source| Error::DownloadInstaller {
            family_name: family_name.to_owned(),
            source,
        })?;

    read_installer_state(
        installer_artifact,
        instance_root,
        libraries_root,
        family_name,
        validate_profile,
    )
    .await
}

pub async fn read_installer_state<V>(
    installer_artifact: &InstallerArtifact,
    instance_root: &Path,
    libraries_root: &Path,
    family_name: &str,
    validate_profile: V,
) -> Result<InstallerPersistedState>
where
    V: Fn(&ForgeInstallerProfile) -> Result<()>,
{
    let archive = InstallerArchive::new(installer_artifact.path.clone());
    let state_paths = InstallerStatePaths::new(instance_root, family_name)?;
    let install_profile = archive
        .read_json::<ForgeInstallerProfile>("install_profile.json")
        .map_err(|source| Error::ReadInstallerProfile {
            family_name: family_name.to_owned(),
            source: Box::new(source),
        })?;
    validate_profile(&install_profile)?;
    let embedded_version = read_embedded_version(&archive, &install_profile, family_name)?;

    state_paths
        .persist_install_profile(&install_profile)
        .await?;
    state_paths
        .persist_embedded_version(embedded_version.as_ref())
        .await?;
    archive
        .extract_maven_artifacts(libraries_root)
        .map_err(|source| Error::ExtractInstallerMavenArtifacts {
            family_name: family_name.to_owned(),
            source: Box::new(source),
        })?;

    Ok(InstallerPersistedState {
        install_profile,
        embedded_version,
    })
}
