use elemental_driver::families::version_json::{
    InstanceStorage, NamedMetadataPatch, PatchMode, PatchTrace, apply_metadata_patches_traced,
    read_base_metadata,
};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    addon::{AddonPatchMode, InstalledAddon},
    error::{Error, Result},
};

use super::{AddonState, load_addon_state, save_addon_state, sync_instance_patches};

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
