use std::path::Path;

use elemental_core::{minecraft::MinecraftVersionId, storage::Storage};
use elemental_schema::mojang::piston::PistonMetaLibraries;

use crate::{
    drivers::vanilla::source::{VanillaSource, resolve_vanilla_metadata},
    error::{Error, Result},
    families::{
        installer::InstallerPersistedState,
        version_json::{
            ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata, VersionJsonInstanceLayout,
            VersionJsonRemoteResolver, VersionJsonRootLayout,
        },
    },
};

use super::{InstallerStatePaths, merge::merge_embedded_version};

pub async fn persist_installer_launch_version<RR, L, VL, F, M>(
    request: InstallerLaunchVersionRequest<'_, RR, L, VL, F, M>,
) -> Result<ResolvedVersionJsonInstance<RR, L, VL>>
where
    RR: VersionJsonRemoteResolver + Clone,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
    F: Fn(Vec<PistonMetaLibraries>) -> Result<Vec<PistonMetaLibraries>>,
    M: Fn(Vec<PistonMetaLibraries>, Vec<PistonMetaLibraries>) -> Vec<PistonMetaLibraries>,
{
    let embedded_version =
        request
            .embedded_version
            .ok_or_else(|| Error::InstallerEmbeddedVersionMissing {
                family_name: request.family_name.to_owned(),
            })?;
    let base_metadata =
        resolve_vanilla_metadata(request.vanilla_source, request.game_version.as_str()).await?;
    let merged_metadata = merge_embedded_version(
        base_metadata.metadata,
        embedded_version,
        request.normalize_libraries,
        request.merge_libraries,
        request.family_name,
    )?;
    ResolvedVersionJsonMetadata::new(
        request.remote_resolver.clone(),
        merged_metadata,
        base_metadata.asset_index_objects,
    )
    .persist(request.instance)
    .await
}

pub struct InstallerLaunchVersionRequest<'a, RR, L, VL, F, M>
where
    RR: VersionJsonRemoteResolver + Clone,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
    F: Fn(Vec<PistonMetaLibraries>) -> Result<Vec<PistonMetaLibraries>>,
    M: Fn(Vec<PistonMetaLibraries>, Vec<PistonMetaLibraries>) -> Vec<PistonMetaLibraries>,
{
    pub instance: &'a Storage<VL, Storage<L>>,
    pub game_version: &'a MinecraftVersionId,
    pub remote_resolver: &'a RR,
    pub vanilla_source: &'a VanillaSource,
    pub embedded_version: Option<&'a serde_json::Value>,
    pub normalize_libraries: F,
    pub merge_libraries: M,
    pub family_name: &'a str,
}

pub fn load_persisted_installer_state(
    instance_root: &Path,
    family_name: &str,
) -> Result<InstallerPersistedState> {
    InstallerStatePaths::new(instance_root, family_name)?.load()
}
