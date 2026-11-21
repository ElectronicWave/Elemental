use elemental_core::minecraft::MinecraftVersionId;
use elemental_core::storage::Storage;
use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    error::{Error, Result},
    families::{
        installer::{
            InstallerArtifact, installer_install_status, profile::load_persisted_install_profile,
        },
        version_json::{
            PreparedVersionJsonInstance, ResolvedVersionJsonInstance, VersionJsonInstanceLayout,
            VersionJsonRootLayout,
        },
    },
};

use super::{InstallerFamily, ResolvedInstallerFamilyVersion};

impl<F, L, VL> ResolvedInstallerFamilyVersion<F, L, VL>
where
    F: InstallerFamily,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    pub async fn load(
        source: &F::Source,
        instance: Storage<VL, Storage<L>>,
    ) -> Result<PreparedVersionJsonInstance<L, VL>> {
        let install_profile = load_persisted_install_profile(&instance.path, F::FAMILY_NAME)?;
        let (game_version, loader_version) = F::profile_identity(&install_profile)?;
        let installer_artifact = F::installer_artifact(
            source,
            &instance.parent,
            &MinecraftVersionId::from(game_version),
            &loader_version,
        )?;
        load_installer_instance(
            instance,
            install_profile,
            installer_artifact,
            F::FAMILY_NAME,
        )
        .await
    }
}

async fn load_installer_instance<L, VL>(
    instance: Storage<VL, Storage<L>>,
    install_profile: ForgeInstallerProfile,
    installer_artifact: InstallerArtifact,
    family_name: &str,
) -> Result<PreparedVersionJsonInstance<L, VL>>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let launch_version = ResolvedVersionJsonInstance::load(instance.clone())?
        .finish_preparation()
        .await?;

    let install_status = installer_install_status(
        &instance,
        &installer_artifact,
        &install_profile,
        launch_version.install_status.is_ready(),
        family_name,
    )
    .await?;

    if !install_status.processors_completed {
        let instance_name = instance.name().ok_or_else(|| Error::MissingInstanceName {
            family_name: family_name.to_owned(),
            path: instance.path.clone(),
        })?;
        return Err(Error::InstallerNotPrepared {
            family_name: family_name.to_owned(),
            instance: instance_name,
            status: format!("{install_status:?}"),
        });
    }

    Ok(launch_version)
}
