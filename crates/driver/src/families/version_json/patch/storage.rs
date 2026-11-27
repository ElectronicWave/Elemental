use std::path::{Path, PathBuf};

use elemental_core::storage::layout::Layoutable;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::Result,
    families::version_json::{
        InstanceStorage, VersionJsonInstanceResource, VersionJsonMetadataStorageExt,
    },
};

use super::{
    NamedMetadataPatch, PatchMode, PatchTrace, apply_metadata_patches,
    apply_metadata_patches_traced,
};

const BASE_VERSION_REL_PATH: &[&str] = &[".elemental", "base-version.json"];

fn baseline_path(instance_root: &Path) -> PathBuf {
    BASE_VERSION_REL_PATH
        .iter()
        .fold(instance_root.to_path_buf(), |path, segment| {
            path.join(segment)
        })
}

/// Reads the clean unpatched [`PistonMetaData`] baseline for `instance`.
///
/// When physical patching has previously materialized `<instance>.json`, this
/// reads `.elemental/base-version.json` instead.
pub fn read_base_metadata(instance: &InstanceStorage) -> Result<PistonMetaData> {
    let baseline = baseline_path(&instance.path);
    if baseline.is_file() {
        let bytes = std::fs::read(&baseline)?;
        return Ok(serde_json::from_slice(&bytes)?);
    }
    instance.metadata()
}

/// Applies scoped patches to `instance`:
/// - Any patches with `effective_mode == PatchMode::Physical` are materialized
///   deterministically from `.elemental/base-version.json` into `<instance>.json`.
///   When no physical patches remain, `<instance>.json` is restored from baseline.
/// - All patches (both `Physical` and `Virtual`) are overlaid in memory from the
///   clean baseline to produce the returned [`PatchTrace`].
pub async fn apply_scoped_patches(
    instance: &InstanceStorage,
    patches: &[NamedMetadataPatch],
) -> Result<PatchTrace> {
    let metadata_path = instance.try_get_resource(VersionJsonInstanceResource::Metadata)?;
    let baseline = baseline_path(&instance.path);
    let physical_patches = patches
        .iter()
        .filter(|patch| patch.effective_mode == PatchMode::Physical)
        .cloned()
        .collect::<Vec<_>>();

    if physical_patches.is_empty() {
        restore_baseline_if_present(&baseline, &metadata_path).await?;
    } else {
        if !baseline.is_file() {
            if let Some(parent) = baseline.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let raw_base = tokio::fs::read(&metadata_path).await?;
            tokio::fs::write(&baseline, raw_base).await?;
        }

        let base_bytes = tokio::fs::read(&baseline).await?;
        let base: PistonMetaData = serde_json::from_slice(&base_bytes)?;
        let merged_physical = apply_metadata_patches(base, &physical_patches)?;
        instance.write_metadata(&merged_physical).await?;
    }

    let base = read_base_metadata(instance)?;
    apply_metadata_patches_traced(base, patches)
}

/// Convenience helper that applies every entry in `patches` using a uniform `mode`.
pub async fn apply_patches_with_mode(
    instance: &InstanceStorage,
    mode: PatchMode,
    patches: &[NamedMetadataPatch],
) -> Result<PistonMetaData> {
    let scoped = patches
        .iter()
        .cloned()
        .map(|mut patch| {
            patch.effective_mode = mode;
            patch
        })
        .collect::<Vec<_>>();
    Ok(apply_scoped_patches(instance, &scoped).await?.metadata)
}

async fn restore_baseline_if_present(baseline: &Path, metadata_path: &Path) -> Result<()> {
    if !baseline.is_file() {
        return Ok(());
    }
    let bytes = tokio::fs::read(baseline).await?;
    tokio::fs::write(metadata_path, bytes).await?;
    tokio::fs::remove_file(baseline).await?;
    Ok(())
}
