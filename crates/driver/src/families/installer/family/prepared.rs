use elemental_core::minecraft::MinecraftVersionId;
use elemental_core::storage::Storage;

use crate::{
    error::{Error, Result},
    families::{
        installer::{
            InstallerArtifact, InstallerPersistedState, installer_install_status,
            load_persisted_installer_state,
        },
        version_json::{
            PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
            prepared::{
                instance::{finish_instance_preparation, load_version_metadata},
                readiness::check_download_status,
            },
        },
    },
};

use super::{InstallerFamily, InstallerFamilyRemoteResolver, ResolvedInstallerFamilyVersion};

impl<F, L, VL> ResolvedInstallerFamilyVersion<F, L, VL>
where
    F: InstallerFamily,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    pub async fn load(
        source: F::Source,
        _remote_resolver: InstallerFamilyRemoteResolver<F>,
        instance: Storage<VL, Storage<L>>,
    ) -> Result<PreparedVersionJsonInstance<L, VL>> {
        let installer_state = load_persisted_installer_state(&instance.path, F::FAMILY_NAME)?;
        let (game_version, loader_version) = F::profile_identity(&installer_state.install_profile)?;
        let installer_artifact = F::installer_artifact(
            &source,
            &instance.parent,
            &MinecraftVersionId::from(game_version),
            &loader_version,
        )?;
        load_installer_instance(
            instance,
            installer_state,
            installer_artifact,
            F::FAMILY_NAME,
        )
        .await
    }
}

async fn load_installer_instance<L, VL>(
    instance: Storage<VL, Storage<L>>,
    installer_state: InstallerPersistedState,
    installer_artifact: InstallerArtifact,
    family_name: &str,
) -> Result<PreparedVersionJsonInstance<L, VL>>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let (metadata, asset_index_objects) = load_version_metadata(&instance)?;
    let download_status = check_download_status(&instance, &metadata, &asset_index_objects)?;
    let launch_version =
        finish_instance_preparation(instance.clone(), metadata, download_status).await?;

    let install_status = installer_install_status(
        &instance,
        &installer_artifact,
        &installer_state.install_profile,
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
