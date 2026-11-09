use elemental_core::minecraft::MinecraftVersionId;
use elemental_core::storage::Storage;

use crate::{
    error::{Error, Result},
    families::{
        installer::{installer_install_status, load_persisted_installer_state},
        version_json::{
            PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
        },
    },
};

use super::{
    InstallerFamily, InstallerFamilyRemoteResolver, ResolvedInstallerFamilyLaunchVersion,
    ResolvedInstallerFamilyVersion,
};

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
    ) -> Result<PreparedVersionJsonInstance<L, VL>> {
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
        .finish_preparation()
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

        Ok(launch_version)
    }
}
