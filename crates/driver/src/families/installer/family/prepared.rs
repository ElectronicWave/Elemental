use std::path::{Path, PathBuf};

use elemental_core::minecraft::MinecraftVersionId;
use elemental_core::storage::Storage;
use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    error::{Error, Result},
    families::{
        installer::{
            InstallerStatePaths, installer_install_status, load_persisted_installer_state,
        },
        version_json::{
            PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
        },
    },
};

use super::{
    InstallerFamily, InstallerFamilyInstallStatus, InstallerFamilyRemoteResolver,
    ResolvedInstallerFamilyLaunchVersion, ResolvedInstallerFamilyVersion,
};

#[derive(Debug, Clone)]
pub struct PreparedInstallerVersion<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub instance: Storage<VL, Storage<L>>,
    pub install_profile: ForgeInstallerProfile,
    pub embedded_version: Option<serde_json::Value>,
    pub launch_version: PreparedVersionJsonInstance<L, VL>,
    pub install_status: InstallerFamilyInstallStatus,
    pub(super) install_profile_path: PathBuf,
    pub(super) embedded_version_path: PathBuf,
}

impl<F, L, VL> ResolvedInstallerFamilyVersion<F, L, VL>
where
    F: InstallerFamily,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    pub async fn load(
        source: F::Source,
        remote_resolver: InstallerFamilyRemoteResolver<F>,
        instance: Storage<VL, Storage<L>>,
    ) -> Result<PreparedInstallerVersion<L, VL>> {
        let state_paths = InstallerStatePaths::new(&instance.path, F::FAMILY_NAME)?;
        let installer_state = load_persisted_installer_state(&instance.path, F::FAMILY_NAME)?;

        let (game_version, loader_version) = F::profile_identity(&installer_state.install_profile)?;
        let resolved_version: ResolvedInstallerFamilyVersion<F, L, VL> =
            ResolvedInstallerFamilyVersion::new(
                source,
                instance.clone(),
                MinecraftVersionId::from(game_version),
                loader_version,
            )?;
        let launch_version = ResolvedInstallerFamilyLaunchVersion::<F, L, VL>::load(
            remote_resolver,
            instance.clone(),
        )?
        .into_prepared()
        .await?;
        let install_status = installer_install_status(
            &resolved_version.instance,
            &resolved_version.installer_artifact,
            &installer_state.install_profile,
            launch_version.install_status.is_ready(),
            F::FAMILY_NAME,
        )
        .await?;

        if !install_status.processors_completed {
            let instance_name = instance.name().ok_or_else(|| Error::MissingInstanceName {
                family_name: F::FAMILY_NAME.to_owned(),
                path: instance.path.clone(),
            })?;
            return Err(Error::InstallerNotPrepared {
                family_name: F::FAMILY_NAME.to_owned(),
                instance: instance_name,
                status: format!("{install_status:?}"),
            });
        }

        Ok(PreparedInstallerVersion {
            instance: resolved_version.instance,
            install_profile: installer_state.install_profile,
            embedded_version: installer_state.embedded_version,
            launch_version,
            install_status,
            install_profile_path: state_paths.install_profile_path(),
            embedded_version_path: state_paths.embedded_version_path(),
        })
    }
}

impl<L, VL> PreparedInstallerVersion<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn instance_root(&self) -> &Path {
        self.instance.path.as_path()
    }

    pub fn install_profile_path(&self) -> PathBuf {
        self.install_profile_path.clone()
    }

    pub fn embedded_version_path(&self) -> PathBuf {
        self.embedded_version_path.clone()
    }

    pub fn required_java_major_version(&self) -> usize {
        self.launch_version.required_java_major_version()
    }
}
