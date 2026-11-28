use std::path::{Component, Path, PathBuf};

use elemental_core::storage::layout::Layoutable;
use elemental_driver::families::version_json::{
    InstanceStorage, NamedMetadataPatch, PatchMode, PatchTrace, VersionJsonInstanceResource,
    apply_metadata_patches_traced, apply_scoped_patches, read_base_metadata,
};
use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::PistonMetaData;
use serde::{Deserialize, Serialize};

pub use crate::addon::InstalledAddonRecord;
use crate::{
    addon::{AddonOrigin, AddonPatchMode, InstalledAddon},
    error::{Error, Result},
};

const ADDONS_STATE_FILE: &str = "addons.json";

/// Persisted addon state stored under `.elemental/addons.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AddonState {
    /// Instance-wide upper-bound patch mode policy (`Virtual` by default).
    #[serde(default)]
    pub patch_mode: PatchMode,
    /// Ordered list of addons installed on the instance.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addons: Vec<InstalledAddonRecord>,
}

impl AddonState {
    /// Inserts or replaces an addon record in installation order.
    pub fn upsert(&mut self, record: InstalledAddonRecord) {
        if let Some(existing) = self.addons.iter_mut().find(|item| item.id == record.id) {
            *existing = record;
        } else {
            self.addons.push(record);
        }
    }

    /// Validates that every Elemental-managed addon's patch mode obeys `self.patch_mode`.
    pub fn validate_modes(&self) -> Result<()> {
        for record in &self.addons {
            if record.origin == AddonOrigin::Elemental {
                record.effective_patch_mode(self.patch_mode)?;
            }
        }
        Ok(())
    }

    /// Resolves all stored [`NamedMetadataPatch`] entries with their effective patch modes.
    pub fn resolve_named_patches(&self) -> Result<Vec<NamedMetadataPatch>> {
        let mut patches = Vec::new();
        for record in &self.addons {
            if record.origin != AddonOrigin::Elemental {
                continue;
            }
            let effective_mode = record.effective_patch_mode(self.patch_mode)?;
            if let Some(patch) = &record.metadata_patch {
                patches.push(NamedMetadataPatch {
                    id: record.id.clone(),
                    effective_mode,
                    patch: patch.clone(),
                });
            }
        }
        Ok(patches)
    }
}

fn state_path(instance: &InstanceStorage) -> Result<PathBuf> {
    let relative = SafeRelativePath::parse(ADDONS_STATE_FILE)?;
    Ok(instance.try_get_resource(VersionJsonInstanceResource::Elemental(Some(relative)))?)
}

/// Loads `.elemental/addons.json` for an instance, returning default state when absent.
pub async fn load_addon_state(instance: &InstanceStorage) -> Result<AddonState> {
    let path = state_path(instance)?;
    match tokio::fs::read(&path).await {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(AddonState::default()),
        Err(err) => Err(err.into()),
    }
}

/// Persists `.elemental/addons.json`, removing the file if state is completely empty default.
pub async fn save_addon_state(instance: &InstanceStorage, state: &AddonState) -> Result<()> {
    let path = state_path(instance)?;
    if state.addons.is_empty() && state.patch_mode == PatchMode::Virtual {
        match tokio::fs::remove_file(&path).await {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(err.into()),
        }
    }

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let bytes = serde_json::to_vec_pretty(state)?;
    tokio::fs::write(&path, bytes).await?;
    Ok(())
}

/// Normalizes a managed file path relative to `instance_root`, rejecting path escapes.
pub fn normalize_managed_file_path(instance_root: &Path, path: &Path) -> Result<String> {
    let relative = if path.is_absolute() {
        path.strip_prefix(instance_root)
            .map_err(|_| Error::UnsafeManagedPath {
                path: path.to_path_buf(),
            })?
            .to_path_buf()
    } else {
        path.to_path_buf()
    };

    if relative.as_os_str().is_empty() {
        return Err(Error::UnsafeManagedPath {
            path: path.to_path_buf(),
        });
    }

    for component in relative.components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(Error::UnsafeManagedPath {
                    path: path.to_path_buf(),
                });
            }
        }
    }

    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn resolve_safe_managed_path(instance_root: &Path, relative: &str) -> Result<PathBuf> {
    let candidate = Path::new(relative);
    if candidate.is_absolute() || candidate.as_os_str().is_empty() {
        return Err(Error::UnsafeManagedPath {
            path: candidate.to_path_buf(),
        });
    }
    let mut joined = instance_root.to_path_buf();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => joined.push(part),
            _ => {
                return Err(Error::UnsafeManagedPath {
                    path: candidate.to_path_buf(),
                });
            }
        }
    }
    Ok(joined)
}

/// Applies all [`AddonState`] metadata patches according to scoped instance & addon policies.
pub async fn sync_instance_patches(
    instance: &InstanceStorage,
    state: &AddonState,
) -> Result<PatchTrace> {
    let patches = state.resolve_named_patches()?;
    Ok(apply_scoped_patches(instance, &patches).await?)
}

/// Replays all active metadata patches against the clean baseline without modifying disk.
pub async fn trace_instance_patches(instance: &InstanceStorage) -> Result<PatchTrace> {
    let state = load_addon_state(instance).await?;
    let patches = state.resolve_named_patches()?;
    let base = read_base_metadata(instance)?;
    Ok(apply_metadata_patches_traced(base, &patches)?)
}

/// Removes an installed addon record, deletes its managed files, and resyncs metadata patches.
pub async fn remove_addon_from_instance(
    instance: &InstanceStorage,
    addon_id: &str,
) -> Result<bool> {
    let mut state = load_addon_state(instance).await?;
    let Some(index) = state.addons.iter().position(|record| record.id == addon_id) else {
        return Ok(false);
    };

    let removed = state.addons.remove(index);
    for relative in &removed.managed_files {
        let target = resolve_safe_managed_path(&instance.path, relative)?;
        match tokio::fs::remove_file(&target).await {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
    }

    save_addon_state(instance, &state).await?;
    sync_instance_patches(instance, &state).await?;
    Ok(true)
}

/// In-memory draft for modifying an instance's addon order and patch modes with zero-I/O
/// [`PatchTrace`] previewing before committing changes to disk.
#[derive(Debug, Clone)]
pub struct AddonModify {
    instance: InstanceStorage,
    base: PistonMetaData,
    original: AddonState,
    draft: AddonState,
    dirty: bool,
}

impl AddonModify {
    /// Loads the clean metadata baseline and persisted addon state for `instance`.
    pub async fn load(instance: InstanceStorage) -> Result<Self> {
        let base = read_base_metadata(&instance)?;
        let state = load_addon_state(&instance).await?;
        Ok(Self {
            instance,
            base,
            original: state.clone(),
            draft: state,
            dirty: false,
        })
    }

    /// Returns the draft instance-wide [`PatchMode`] policy.
    pub fn patch_mode(&self) -> PatchMode {
        self.draft.patch_mode
    }

    /// Returns the current draft addon list with effective patch modes resolved.
    pub fn addons(&self) -> Vec<InstalledAddon> {
        self.draft
            .addons
            .iter()
            .map(|record| record.to_installed(self.draft.patch_mode))
            .collect()
    }

    /// Returns `true` when the draft has uncommitted modifications.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Computes the current [`PatchTrace`] in memory without touching disk.
    pub fn trace(&self) -> Result<PatchTrace> {
        trace_state_with_extra(&self.base, &self.draft, &[])
    }

    /// Computes the [`PatchTrace`] in memory with additional ephemeral patches appended
    /// (for example launch-scoped addons such as `authlib-injector`).
    pub fn trace_with_extra(&self, extra_patches: &[NamedMetadataPatch]) -> Result<PatchTrace> {
        trace_state_with_extra(&self.base, &self.draft, extra_patches)
    }

    /// Updates the preferred addon order in the draft and returns the resulting [`PatchTrace`].
    ///
    /// IDs listed in `ordered_ids` are placed first in that exact order; any remaining
    /// installed addons retain their existing relative order afterwards.
    pub fn reorder(&mut self, ordered_ids: &[&str]) -> Result<PatchTrace> {
        for id in ordered_ids {
            if !self.draft.addons.iter().any(|record| record.id == *id) {
                return Err(Error::UnknownAddonInOrder {
                    id: (*id).to_owned(),
                });
            }
        }

        let mut next = self.draft.clone();
        let mut reordered = Vec::with_capacity(next.addons.len());
        let mut remaining = std::mem::take(&mut next.addons);

        for id in ordered_ids {
            if let Some(pos) = remaining.iter().position(|record| record.id == *id) {
                reordered.push(remaining.remove(pos));
            }
        }
        reordered.extend(remaining);
        next.addons = reordered;

        let trace = trace_state_with_extra(&self.base, &next, &[])?;
        self.draft = next;
        self.dirty = true;
        Ok(trace)
    }

    /// Updates the draft instance-wide [`PatchMode`] policy and returns the resulting [`PatchTrace`].
    pub fn set_instance_patch_mode(&mut self, mode: PatchMode) -> Result<PatchTrace> {
        let mut next = self.draft.clone();
        next.patch_mode = mode;
        next.validate_modes()?;
        let trace = trace_state_with_extra(&self.base, &next, &[])?;
        self.draft = next;
        self.dirty = true;
        Ok(trace)
    }

    /// Updates a single addon's [`AddonPatchMode`] preference in the draft and returns the resulting [`PatchTrace`].
    pub fn set_addon_patch_mode(
        &mut self,
        addon_id: &str,
        mode: AddonPatchMode,
    ) -> Result<PatchTrace> {
        let mut next = self.draft.clone();
        let Some(record) = next.addons.iter_mut().find(|record| record.id == addon_id) else {
            return Err(Error::AddonNotInstalled {
                id: addon_id.to_owned(),
            });
        };
        record.patch_mode = mode;
        next.validate_modes()?;
        let trace = trace_state_with_extra(&self.base, &next, &[])?;
        self.draft = next;
        self.dirty = true;
        Ok(trace)
    }

    /// Discards all uncommitted draft changes and returns the restored [`PatchTrace`].
    pub fn discard(&mut self) -> Result<PatchTrace> {
        self.draft = self.original.clone();
        self.dirty = false;
        self.trace()
    }

    /// Persists the draft addon state and syncs physical/virtual metadata patches to disk.
    pub async fn commit(&mut self) -> Result<PatchTrace> {
        let trace = sync_instance_patches(&self.instance, &self.draft).await?;
        save_addon_state(&self.instance, &self.draft).await?;
        self.original = self.draft.clone();
        self.dirty = false;
        Ok(trace)
    }
}

fn trace_state_with_extra(
    base: &PistonMetaData,
    state: &AddonState,
    extra_patches: &[NamedMetadataPatch],
) -> Result<PatchTrace> {
    let mut patches = state.resolve_named_patches()?;
    patches.extend(extra_patches.iter().cloned());
    Ok(apply_metadata_patches_traced(base.clone(), &patches)?)
}

/// Reorders installed addons on `instance` and resyncs metadata patches.
///
/// IDs listed in `ordered_ids` are placed first in that exact order; any remaining
/// installed addons retain their existing relative order afterwards.
pub async fn reorder_instance_addons(
    instance: &InstanceStorage,
    ordered_ids: &[&str],
) -> Result<PatchTrace> {
    let mut modify = AddonModify::load(instance.clone()).await?;
    modify.reorder(ordered_ids)?;
    modify.commit().await
}

/// Switches an instance's [`PatchMode`] policy (`Virtual` <-> `Physical`) and resyncs metadata.
pub async fn set_instance_patch_mode(
    instance: &InstanceStorage,
    mode: PatchMode,
) -> Result<PatchTrace> {
    let mut modify = AddonModify::load(instance.clone()).await?;
    modify.set_instance_patch_mode(mode)?;
    modify.commit().await
}

/// Updates an individual addon's [`AddonPatchMode`] preference and resyncs metadata.
pub async fn set_addon_patch_mode(
    instance: &InstanceStorage,
    addon_id: &str,
    mode: AddonPatchMode,
) -> Result<PatchTrace> {
    let mut modify = AddonModify::load(instance.clone()).await?;
    modify.set_addon_patch_mode(addon_id, mode)?;
    modify.commit().await
}
