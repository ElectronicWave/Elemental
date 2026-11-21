use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    error::{Error, Result},
    families::{
        installer::{
            InstallerMavenSource, installer_install_status, profile::load_persisted_install_profile,
        },
        version_json::{InstanceStorage, PreparedVersionJsonInstance, ResolvedVersionJsonInstance},
    },
};

use super::ResolvedInstallerFamilyVersion;

impl ResolvedInstallerFamilyVersion {
    pub async fn load(
        source: &InstallerMavenSource,
        instance: InstanceStorage,
    ) -> Result<PreparedVersionJsonInstance> {
        let spec = source.spec();
        let family_name = spec.family_name;
        let install_profile = load_persisted_install_profile(&instance.path, family_name)?;
        let (game_version, loader_version) = spec.profile_identity(&install_profile)?;
        let installer_artifact = source.installer_artifact(
            &instance.parent,
            &MinecraftVersionId::from(game_version),
            &loader_version,
        )?;

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
}
