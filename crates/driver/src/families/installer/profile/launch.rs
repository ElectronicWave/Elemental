use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::{forge::ForgeInstallerProfile, mojang::piston::PistonMetaLibraries};
use serde_json::value::RawValue;

use crate::{
    error::{Error, Result},
    families::{
        installer::InstallerPersistedState,
        version_json::{
            InstanceStorage, ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata,
            VanillaSource, resolve_vanilla_metadata,
        },
    },
};

use super::{InstallerStatePaths, merge::merge_embedded_version};

pub async fn persist_installer_launch_version(
    instance: &InstanceStorage,
    game_version: &MinecraftVersionId,
    vanilla_source: &VanillaSource,
    embedded_version: Option<&RawValue>,
    normalize_libraries: &(
         dyn Fn(Vec<PistonMetaLibraries>) -> Result<Vec<PistonMetaLibraries>> + Send + Sync
     ),
    merge_libraries: &(
         dyn Fn(Vec<PistonMetaLibraries>, Vec<PistonMetaLibraries>) -> Vec<PistonMetaLibraries>
             + Send
             + Sync
     ),
    family_name: &str,
) -> Result<ResolvedVersionJsonInstance> {
    let embedded_version =
        embedded_version.ok_or_else(|| Error::InstallerEmbeddedVersionMissing {
            family_name: family_name.to_owned(),
        })?;
    let base_metadata = resolve_vanilla_metadata(vanilla_source, game_version.as_str()).await?;
    let merged_metadata = merge_embedded_version(
        base_metadata.metadata,
        embedded_version,
        normalize_libraries,
        merge_libraries,
        family_name,
    )?;
    ResolvedVersionJsonMetadata::new(merged_metadata, base_metadata.asset_index_objects)
        .persist(instance)
        .await
}

pub fn load_persisted_installer_state(
    instance: &InstanceStorage,
    family_name: &str,
) -> Result<InstallerPersistedState> {
    InstallerStatePaths::new(instance, family_name)?.load()
}

pub fn load_persisted_install_profile(
    instance: &InstanceStorage,
    family_name: &str,
) -> Result<ForgeInstallerProfile> {
    InstallerStatePaths::new(instance, family_name)?.load_install_profile()
}
