use std::sync::Arc;

use elemental_addon::{
    Addon, AddonCompatibility, AddonContext, AddonLaunchArgs, AddonLaunchContext, AddonOrigin,
    AddonPatchMode, AddonPrepareRequest, InstalledAddon, InstalledAddonRecord, NamedMetadataPatch,
    PatchMode, PatchTrace, load_addon_state, normalize_managed_file_path,
    resolve_effective_patch_mode, save_addon_state, sync_instance_patches,
};
use elemental_core::{
    auth::credential::Credential, minecraft::MinecraftVersionId, runtime::RuntimeResolver,
};
use elemental_driver::{
    driver::{DriverDescriptor, InstanceProbe},
    families::version_json::{
        InstanceStorage, PreparedVersionJsonInstance, apply_metadata_patches_traced,
        read_base_metadata,
    },
};
use elemental_infra::downloader::{DownloadSession, ElementalDownloader};

use crate::{
    addon::{AddonRegistry, AddonSpec},
    error::{Error, Result},
    request::{LaunchOptions, RuntimeOptions},
    result::PreparedInstance,
};

pub(super) enum RequestedAddon {
    BySpec(AddonSpec),
    Direct {
        addon: Arc<dyn Addon>,
        version: Option<String>,
        patch_mode: AddonPatchMode,
    },
}

impl RequestedAddon {
    pub(super) fn direct(
        addon: impl Addon + 'static,
        version: Option<String>,
        patch_mode: AddonPatchMode,
    ) -> Self {
        Self::Direct {
            addon: Arc::new(addon),
            version,
            patch_mode,
        }
    }
}

pub(super) struct AddonPreparationContext<'a> {
    pub(super) session: &'a DownloadSession,
    pub(super) instance: &'a InstanceStorage,
    pub(super) driver: DriverDescriptor,
    pub(super) game_version: &'a MinecraftVersionId,
    pub(super) runtime_options: &'a RuntimeOptions,
    pub(super) runtime_resolver: &'a RuntimeResolver,
}

pub(super) struct LaunchHookEntry {
    pub(super) record: InstalledAddonRecord,
    pub(super) addon: Option<Arc<dyn Addon>>,
    pub(super) effective_mode: PatchMode,
}

pub(super) async fn apply_requested_addons(
    addons: &AddonRegistry,
    ctx: &AddonPreparationContext<'_>,
    prepared: &mut PreparedVersionJsonInstance,
    requested_addons: Vec<RequestedAddon>,
    patch_mode: Option<PatchMode>,
) -> Result<(PatchMode, Vec<InstalledAddonRecord>)> {
    let mut state = load_addon_state(ctx.instance).await?;
    let mode_changed = patch_mode.is_some_and(|mode| mode != state.patch_mode);
    if let Some(mode) = patch_mode {
        state.patch_mode = mode;
    }

    if requested_addons.is_empty()
        && !mode_changed
        && state.addons.is_empty()
        && state.patch_mode == PatchMode::Virtual
    {
        return Ok((PatchMode::Virtual, Vec::new()));
    }

    for requested in requested_addons {
        let (record, _) = prepare_addon_record(addons, ctx, state.patch_mode, requested).await?;
        state.upsert(record);
    }

    state.validate_modes()?;
    let trace = sync_instance_patches(ctx.instance, &state).await?;
    save_addon_state(ctx.instance, &state).await?;
    prepared.metadata = trace.metadata;
    Ok((state.patch_mode, state.addons))
}

pub(super) async fn prepare_addon_record(
    addons: &AddonRegistry,
    ctx: &AddonPreparationContext<'_>,
    instance_policy: PatchMode,
    requested: RequestedAddon,
) -> Result<(InstalledAddonRecord, Arc<dyn Addon>)> {
    let context = AddonContext {
        driver: ctx.driver,
        game_version: Some(ctx.game_version),
        instance: ctx.instance,
    };

    let (addon, version, addon_patch_mode) = match requested {
        RequestedAddon::BySpec(spec) => {
            let addon = addons.addon(&spec.id)?;
            (addon, spec.version, spec.patch_mode)
        }
        RequestedAddon::Direct {
            addon,
            version,
            patch_mode,
        } => (addon, version, patch_mode),
    };

    let descriptor = addon.descriptor();
    let support = addon.patch_mode_support();
    resolve_effective_patch_mode(&descriptor.id, instance_policy, addon_patch_mode, support)?;

    if let AddonCompatibility::Incompatible { reason } = addon.check_compatibility(&context) {
        return Err(Error::AddonIncompatible {
            id: descriptor.id.into_owned(),
            reason,
        });
    }

    let output = addon
        .prepare(&AddonPrepareRequest {
            session: ctx.session,
            instance: ctx.instance,
            driver: ctx.driver,
            game_version: ctx.game_version,
            addon_version: version.as_deref(),
            runtime_options: ctx.runtime_options,
            runtime_resolver: ctx.runtime_resolver,
        })
        .await?;

    let managed_files = output
        .managed_files
        .iter()
        .map(|path| normalize_managed_file_path(&ctx.instance.path, path))
        .collect::<elemental_addon::Result<Vec<String>>>()?;

    let record = InstalledAddonRecord {
        id: descriptor.id.into_owned(),
        name: descriptor.name.into_owned(),
        version: output.version.or(version),
        origin: AddonOrigin::Elemental,
        patch_mode: addon_patch_mode,
        patch_mode_support: support,
        requires_runtime_hook: addon.requires_runtime_hook(),
        managed_files,
        metadata_patch: output.metadata_patch,
    };
    Ok((record, addon))
}

pub(super) async fn resolve_launch_addons(
    addons: &AddonRegistry,
    downloader: &ElementalDownloader,
    runtime_resolver: &RuntimeResolver,
    prepared: &PreparedInstance,
    runtime_options: &RuntimeOptions,
    session: Option<DownloadSession>,
    launch_addons: Vec<RequestedAddon>,
) -> Result<(Vec<LaunchHookEntry>, Option<PatchTrace>)> {
    let mut entries = collect_persistent_launch_entries(addons, prepared)?;
    let pending = dedup_launch_addons(&entries, launch_addons);

    if pending.is_empty() {
        if let Some(session) = session {
            session.finish().await?.success()?;
        }
        return Ok((entries, None));
    }

    let session = match session {
        Some(session) => session,
        None => {
            downloader
                .session(format!(
                    "prepare launch addons {}",
                    prepared.launch_version.metadata.id
                ))
                .open()
                .await?
        }
    };

    let ctx = AddonPreparationContext {
        session: &session,
        instance: &prepared.launch_version.version,
        driver: prepared.driver,
        game_version: &prepared.game_version,
        runtime_options,
        runtime_resolver,
    };
    let ephemeral_result = prepare_ephemeral_launch_entries(addons, &ctx, pending).await;

    let report = session.finish().await;
    let ephemeral_entries = ephemeral_result?;
    report?.success()?;

    entries.extend(ephemeral_entries);
    let trace = order_and_trace_launch_entries(prepared, &mut entries)?;
    Ok((entries, Some(trace)))
}

fn collect_persistent_launch_entries(
    addons: &AddonRegistry,
    prepared: &PreparedInstance,
) -> Result<Vec<LaunchHookEntry>> {
    prepared
        .addons
        .iter()
        .filter(|record| record.origin == AddonOrigin::Elemental)
        .map(|record| {
            let effective_mode = record.effective_patch_mode(prepared.patch_mode)?;
            Ok(LaunchHookEntry {
                addon: addons.get(&record.id),
                record: record.clone(),
                effective_mode,
            })
        })
        .collect()
}

async fn prepare_ephemeral_launch_entries(
    addons: &AddonRegistry,
    ctx: &AddonPreparationContext<'_>,
    pending: Vec<RequestedAddon>,
) -> Result<Vec<LaunchHookEntry>> {
    let mut entries = Vec::with_capacity(pending.len());
    for requested in pending {
        let (record, addon) =
            prepare_addon_record(addons, ctx, PatchMode::Virtual, requested).await?;
        entries.push(LaunchHookEntry {
            record,
            addon: Some(addon),
            effective_mode: PatchMode::Virtual,
        });
    }
    Ok(entries)
}

pub(super) fn ensure_required_hooks_registered(
    addons: &AddonRegistry,
    records: &[InstalledAddonRecord],
) -> Result<()> {
    for record in records {
        if record.origin == AddonOrigin::Elemental
            && record.requires_runtime_hook
            && addons.get(&record.id).is_none()
        {
            return Err(Error::AddonNotRegistered {
                id: record.id.clone(),
            });
        }
    }
    Ok(())
}

pub(super) async fn apply_launch_addon_hooks(
    prepared: &PreparedInstance,
    entries: &[LaunchHookEntry],
    credential: &Credential,
    options: &mut LaunchOptions,
) -> Result<()> {
    for entry in entries {
        let Some(addon) = &entry.addon else {
            if entry.record.requires_runtime_hook {
                return Err(Error::AddonNotRegistered {
                    id: entry.record.id.clone(),
                });
            }
            continue;
        };

        let mut args = AddonLaunchArgs::default();
        addon
            .augment_launch(
                &mut args,
                &AddonLaunchContext {
                    instance: &prepared.launch_version.version,
                    driver: prepared.driver,
                    credential,
                    record: &entry.record,
                },
            )
            .await?;
        options.extra_jvm_arguments.extend(args.extra_jvm_arguments);
        options
            .extra_game_arguments
            .extend(args.extra_game_arguments);
    }
    Ok(())
}

pub(super) async fn detect_instance_addons(
    addons: &AddonRegistry,
    storage: &InstanceStorage,
) -> Result<Vec<InstalledAddon>> {
    let state = load_addon_state(storage).await?;
    let mut detected_addons = state
        .addons
        .iter()
        .map(|record| record.to_installed(state.patch_mode))
        .collect::<Vec<_>>();

    if addons.iter().next().is_none() {
        return Ok(detected_addons);
    }

    let probe = InstanceProbe::collect(storage).await?;
    for addon in addons.iter() {
        let descriptor = addon.descriptor();
        if detected_addons.iter().any(|item| item.id == descriptor.id) {
            continue;
        }
        if let Some(detected) = addon.inspect(&probe).await? {
            detected_addons.push(detected);
        }
    }
    Ok(detected_addons)
}

fn dedup_launch_addons(
    persistent: &[LaunchHookEntry],
    launch_addons: Vec<RequestedAddon>,
) -> Vec<RequestedAddon> {
    let mut pending = Vec::with_capacity(launch_addons.len());
    let mut seen_ids = persistent
        .iter()
        .map(|entry| entry.record.id.clone())
        .collect::<Vec<_>>();
    for requested in launch_addons {
        let id = match &requested {
            RequestedAddon::BySpec(spec) => spec.id.clone(),
            RequestedAddon::Direct { addon, .. } => addon.descriptor().id.into_owned(),
        };
        if seen_ids.iter().any(|seen| seen == &id) {
            continue;
        }
        seen_ids.push(id);
        pending.push(requested);
    }
    pending
}

pub(super) fn order_and_trace_launch_entries(
    prepared: &PreparedInstance,
    entries: &mut Vec<LaunchHookEntry>,
) -> Result<PatchTrace> {
    let base = read_base_metadata(&prepared.launch_version.version)?;
    let patches = entries
        .iter()
        .map(|entry| NamedMetadataPatch {
            id: entry.record.id.clone(),
            effective_mode: entry.effective_mode,
            patch: entry.record.metadata_patch.clone().unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    let mut trace = apply_metadata_patches_traced(base, &patches)?;
    let mut remaining = std::mem::take(entries);
    let mut ordered = Vec::with_capacity(remaining.len());
    for step in &trace.steps {
        if let Some(pos) = remaining
            .iter()
            .position(|entry| entry.record.id == step.id)
        {
            ordered.push(remaining.remove(pos));
        }
    }
    ordered.extend(remaining);
    *entries = ordered;

    trace.steps.retain(|step| {
        entries
            .iter()
            .find(|entry| entry.record.id == step.id)
            .is_some_and(|entry| entry.record.metadata_patch.is_some())
    });

    Ok(trace)
}
