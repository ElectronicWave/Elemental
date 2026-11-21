use elemental_infra::SafeRelativePath;
use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    error::Result,
    families::{
        installer::{
            InstallerArtifact, installer_client_processors_ready,
            libraries::installer_library_artifacts,
        },
        version_json::{InstanceStorage, storage::library_artifacts_present},
    },
};

use super::{InstallerInstallStatus, InstallerStatePaths};

fn profile_libraries_ready(
    instance: &InstanceStorage,
    install_profile: &ForgeInstallerProfile,
) -> Result<bool> {
    let mut artifact_paths = Vec::new();
    for library in &install_profile.libraries {
        for artifact in installer_library_artifacts(library)? {
            artifact_paths.push(SafeRelativePath::parse(artifact.path.as_str())?);
        }
    }
    library_artifacts_present(&instance.parent, artifact_paths)
}

pub async fn installer_install_status(
    instance: &InstanceStorage,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
    launch_version_ready: bool,
    family_name: &str,
) -> Result<InstallerInstallStatus> {
    let state_paths = InstallerStatePaths::new(&instance.path, family_name)?;

    Ok(InstallerInstallStatus {
        installer_downloaded: installer_artifact.path.exists(),
        install_profile_persisted: state_paths.install_profile_path().exists(),
        embedded_version_persisted: state_paths.embedded_version_path().exists(),
        profile_libraries_ready: profile_libraries_ready(instance, install_profile)?,
        processors_completed: installer_client_processors_ready(
            instance,
            installer_artifact,
            install_profile,
            family_name,
        )
        .await?,
        launch_version_ready,
    })
}
