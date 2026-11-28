#![cfg_attr(not(feature = "driver-all"), allow(dead_code, unreachable_code))]
#![allow(unreachable_patterns)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use elemental_addon::{
    Addon, AddonCompatibility, AddonContext, AddonLaunchArgs, AddonLaunchContext, AddonOrigin,
    AddonPatchMode, AddonPrepareRequest, InstalledAddon, InstalledAddonRecord, NamedMetadataPatch,
    PatchMode, PatchTrace, PistonMetaData, load_addon_state, normalize_managed_file_path,
    remove_addon_from_instance, reorder_instance_addons, resolve_effective_patch_mode,
    save_addon_state, set_addon_patch_mode, set_instance_patch_mode, sync_instance_patches,
    trace_instance_patches,
};
use elemental_core::{
    auth::credential::Credential,
    http::SwappableClient,
    launcher::{command::LaunchCommand, process},
    minecraft::MinecraftVersionId,
    runtime::RuntimeResolver,
    storage::Storage,
};
use elemental_driver::{
    driver::{
        Driver, DriverDescriptor, InstalledInstance, InstanceProbe, PrepareRequest, PrepareStage,
        inspect_instances,
    },
    families::version_json::{
        GameStorage, InstanceStorage, SharedInstanceLayout, SharedRootLayout,
        VersionJsonGameStorageExt, apply_metadata_patches, read_base_metadata,
    },
};
use elemental_infra::downloader::{DownloadSession, ElementalDownloader};

use crate::{
    addon::{AddonRegistry, AddonSpec},
    error::{Error, Result},
    origin::DriverOrigins,
    registration::{DriverRegistration, DriverRegistry},
    request::{LaunchOptions, RuntimeOptions},
    result::{
        InspectReport, Instance, InstanceInspectionFailure, LaunchedInstance, PreparedInstance,
        RemoveInstanceOutcome,
    },
    spec::DriverSpec,
};

#[derive(Debug, Clone)]
pub struct Launcher {
    storage_root: PathBuf,
    http_client: SwappableClient,
    downloader: ElementalDownloader,
    root_layout: SharedRootLayout,
    instance_layout: SharedInstanceLayout,
    origins: DriverOrigins,
    runtime_resolver: RuntimeResolver,
    drivers: DriverRegistry,
    addons: AddonRegistry,
}

impl Launcher {
    pub(crate) fn new(
        storage_root: PathBuf,
        http_client: SwappableClient,
        downloader: ElementalDownloader,
        root_layout: SharedRootLayout,
        instance_layout: SharedInstanceLayout,
        origins: DriverOrigins,
        runtime_resolver: RuntimeResolver,
    ) -> Self {
        let drivers = DriverRegistry::new(http_client.clone());
        let addons = AddonRegistry::new(&storage_root, http_client.clone(), &origins);
        Self {
            storage_root,
            http_client,
            downloader,
            root_layout,
            instance_layout,
            origins,
            runtime_resolver,
            drivers,
            addons,
        }
    }

    /// Adds a third-party driver after the launcher has been built.
    ///
    /// Built-in descriptors cannot be replaced. Use [`DriverSpec::custom`] to
    /// prepare an instance for a registered third-party driver.
    pub fn register_driver(mut self, registration: DriverRegistration) -> Result<Self> {
        self.drivers.register(registration)?;
        Ok(self)
    }

    /// Registers a third-party addon on this launcher.
    pub fn register_addon(mut self, addon: impl Addon + 'static) -> Result<Self> {
        self.addons.register(Arc::new(addon))?;
        Ok(self)
    }

    pub fn drivers(&self) -> &DriverRegistry {
        &self.drivers
    }

    pub fn addons(&self) -> &AddonRegistry {
        &self.addons
    }

    pub fn storage_root(&self) -> &Path {
        &self.storage_root
    }

    pub fn downloader(&self) -> &ElementalDownloader {
        &self.downloader
    }

    /// Returns the shared [`SwappableClient`] used by built-in drivers,
    /// catalogs, and the default downloader.
    pub fn http_client(&self) -> &SwappableClient {
        &self.http_client
    }

    pub fn runtime_resolver(&self) -> &RuntimeResolver {
        &self.runtime_resolver
    }

    pub async fn inspect_instances(&self) -> Result<InspectReport> {
        let storage = self.game_storage();
        let drivers = self.inspect_drivers()?;
        let driver_refs = driver_refs(&drivers);
        let detected =
            inspect_instances(&storage, self.instance_layout.clone(), &driver_refs).await?;
        let mut instances = Vec::new();
        let mut failures = Vec::new();
        for result in detected {
            match result {
                Ok(instance) => {
                    let addons = self.detect_instance_addons(&instance.storage).await?;
                    instances.push(summarize_installed_instance(instance, addons));
                }
                Err(failure) => failures.push(InstanceInspectionFailure {
                    instance_name: failure.instance_name,
                    path: failure.path,
                    error: failure.error.into(),
                }),
            }
        }
        instances.sort_by(|left, right| left.instance_name.cmp(&right.instance_name));
        Ok(InspectReport {
            instances,
            failures,
        })
    }

    pub async fn inspect_instance(&self, instance_name: String) -> Result<Option<Instance>> {
        let instance = self.instance(instance_name)?;
        let drivers = self.inspect_drivers()?;
        let driver_refs = driver_refs(&drivers);

        match InstalledInstance::detect(instance, &driver_refs).await? {
            Some(installed) => {
                let addons = self.detect_instance_addons(&installed.storage).await?;
                Ok(Some(summarize_installed_instance(installed, addons)))
            }
            None => Ok(None),
        }
    }

    /// Removes an installed instance and its root directory.
    ///
    /// This does not require the instance metadata to be valid, so corrupted
    /// installations can also be cleaned up.
    pub async fn remove_instance(
        &self,
        instance_name: impl Into<String>,
    ) -> Result<RemoveInstanceOutcome> {
        let instance_name = instance_name.into();
        let path = self.game_storage().instance_root(instance_name)?;
        match tokio::fs::remove_dir_all(&path).await {
            Ok(()) => Ok(RemoveInstanceOutcome::Removed),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                Ok(RemoveInstanceOutcome::NotFound)
            }
            Err(source) => Err(source.into()),
        }
    }

    /// Starts configuring an instance preparation operation.
    pub fn prepare_instance(
        &self,
        instance_name: impl Into<String>,
        driver: DriverSpec,
    ) -> PrepareInstanceBuilder<'_> {
        PrepareInstanceBuilder {
            launcher: self,
            instance_name: instance_name.into(),
            driver,
            runtime_options: None,
            session: None,
            on_stage: None,
            addons: Vec::new(),
            patch_mode: None,
        }
    }

    /// Starts configuring addon installation onto an existing local instance without
    /// re-running driver preparation.
    pub fn install_addons(&self, instance_name: impl Into<String>) -> InstallAddonsBuilder<'_> {
        InstallAddonsBuilder {
            launcher: self,
            instance_name: instance_name.into(),
            runtime_options: None,
            session: None,
            addons: Vec::new(),
            patch_mode: None,
        }
    }

    /// Install or refresh an instance with a default download session.
    pub async fn prepare(
        &self,
        instance_name: String,
        driver: DriverSpec,
        runtime_options: &RuntimeOptions,
    ) -> Result<PreparedInstance> {
        self.prepare_instance(instance_name, driver)
            .runtime_options(runtime_options)
            .run()
            .await
    }

    /// Install or refresh an instance with a caller-configured session.
    ///
    /// Observe progress or download events through the session before passing it
    /// here. The launcher then finishes and validates the session before
    /// returning.
    pub async fn prepare_with_session(
        &self,
        session: DownloadSession,
        instance_name: String,
        driver: DriverSpec,
        runtime_options: &RuntimeOptions,
    ) -> Result<PreparedInstance> {
        self.prepare_instance(instance_name, driver)
            .session(session)
            .runtime_options(runtime_options)
            .run()
            .await
    }

    /// Low-level preparation inside a caller-owned session.
    ///
    /// The caller must finish or cancel `session` and validate its report.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn prepare_in_session(
        &self,
        session: &DownloadSession,
        instance_name: String,
        driver: DriverSpec,
        runtime_options: &RuntimeOptions,
        on_stage: Option<&(dyn Fn(PrepareStage) + Send + Sync)>,
        requested_addons: Vec<RequestedAddon>,
        patch_mode: Option<PatchMode>,
    ) -> Result<PreparedInstance> {
        let instance = self
            .game_storage()
            .ensure_instance(instance_name, self.instance_layout.clone())
            .await?;
        let resolved_driver = self.resolve_driver(driver.descriptor())?;
        let mut prepared = resolved_driver
            .prepare(&PrepareRequest {
                session,
                instance: &instance,
                game_version: driver.game_version(),
                loader_version: driver.loader_version(),
                runtime_options,
                runtime_resolver: &self.runtime_resolver,
                on_stage,
            })
            .await?;
        let (effective_policy, addons) = self
            .apply_requested_addons(
                session,
                &instance,
                driver.descriptor(),
                driver.game_version(),
                runtime_options,
                &mut prepared,
                requested_addons,
                patch_mode,
            )
            .await?;
        Ok(PreparedInstance::new(
            driver.descriptor(),
            driver.game_version().clone(),
            prepared,
            effective_policy,
            addons,
        ))
    }

    pub(crate) async fn install_addons_in_session(
        &self,
        session: &DownloadSession,
        instance_name: String,
        runtime_options: &RuntimeOptions,
        requested_addons: Vec<RequestedAddon>,
        patch_mode: Option<PatchMode>,
    ) -> Result<PreparedInstance> {
        let inspected = self
            .inspect_instance(instance_name.clone())
            .await?
            .ok_or_else(|| Error::InstanceNotFound {
                name: instance_name.clone(),
            })?;
        let layout = self.instance(instance_name)?;
        let driver_descriptor = inspected.driver.driver;
        let mut prepared = self
            .resolve_driver(driver_descriptor)?
            .load_prepared(&layout)
            .await?;
        let game_version = resolve_instance_game_version(&layout, inspected.driver.game_version)?;

        let (effective_policy, addons) = self
            .apply_requested_addons(
                session,
                &layout,
                driver_descriptor,
                &game_version,
                runtime_options,
                &mut prepared,
                requested_addons,
                patch_mode,
            )
            .await?;
        Ok(PreparedInstance::new(
            driver_descriptor,
            game_version,
            prepared,
            effective_policy,
            addons,
        ))
    }

    pub async fn load_instance(&self, instance: Instance) -> Result<PreparedInstance> {
        let layout = self.instance(instance.instance_name)?;
        let driver_spec = instance.driver.driver;
        let game_version = resolve_instance_game_version(&layout, instance.driver.game_version)?;
        let mut prepared = self
            .resolve_driver(driver_spec)?
            .load_prepared(&layout)
            .await?;
        let state = load_addon_state(&layout).await?;
        self.ensure_required_hooks_registered(&state.addons)?;
        if !state.addons.is_empty() || state.patch_mode == PatchMode::Physical {
            prepared.metadata = sync_instance_patches(&layout, &state).await?.metadata;
        }
        Ok(PreparedInstance::new(
            driver_spec,
            game_version,
            prepared,
            state.patch_mode,
            state.addons,
        ))
    }

    /// Returns all addons installed or detected on the named instance.
    pub async fn instance_addons(
        &self,
        instance_name: impl Into<String>,
    ) -> Result<Vec<InstalledAddon>> {
        let layout = self.instance(instance_name.into())?;
        self.detect_instance_addons(&layout).await
    }

    pub async fn remove_addon(
        &self,
        instance_name: impl Into<String>,
        addon_id: &str,
    ) -> Result<bool> {
        let layout = self.instance(instance_name.into())?;
        Ok(remove_addon_from_instance(&layout, addon_id).await?)
    }

    pub async fn set_instance_patch_mode(
        &self,
        instance_name: impl Into<String>,
        mode: PatchMode,
    ) -> Result<PatchTrace> {
        let layout = self.instance(instance_name.into())?;
        Ok(set_instance_patch_mode(&layout, mode).await?)
    }

    pub async fn set_addon_patch_mode(
        &self,
        instance_name: impl Into<String>,
        addon_id: &str,
        mode: AddonPatchMode,
    ) -> Result<PatchTrace> {
        let layout = self.instance(instance_name.into())?;
        Ok(set_addon_patch_mode(&layout, addon_id, mode).await?)
    }

    pub async fn reorder_addons(
        &self,
        instance_name: impl Into<String>,
        ordered_ids: &[&str],
    ) -> Result<PatchTrace> {
        let layout = self.instance(instance_name.into())?;
        Ok(reorder_instance_addons(&layout, ordered_ids).await?)
    }

    pub async fn trace_instance_patches(
        &self,
        instance_name: impl Into<String>,
    ) -> Result<PatchTrace> {
        let layout = self.instance(instance_name.into())?;
        Ok(trace_instance_patches(&layout).await?)
    }

    pub async fn build_launch_command(
        &self,
        prepared: &PreparedInstance,
        credential: &Credential,
        runtime_options: &RuntimeOptions,
        options: &LaunchOptions,
    ) -> Result<LaunchCommand> {
        let runtime = prepared
            .resolve_runtime(runtime_options, &self.runtime_resolver)
            .await?;
        let (launch_records, metadata_override) = self
            .prepare_launch_addons(prepared, runtime_options, options)
            .await?;
        let mut options = options.clone();
        self.apply_launch_addon_hooks(prepared, &launch_records, credential, &mut options)
            .await?;
        prepared.build_launch_command(credential, runtime, &options, metadata_override)
    }

    pub fn launch(&self, command: LaunchCommand) -> Result<LaunchedInstance> {
        let runtime = command.runtime.clone();
        let child = process::spawn_command(command)?;
        Ok(LaunchedInstance { runtime, child })
    }

    pub async fn launch_prepared_instance(
        &self,
        prepared: &PreparedInstance,
        credential: &Credential,
        runtime_options: &RuntimeOptions,
        options: &LaunchOptions,
    ) -> Result<LaunchedInstance> {
        let command = self
            .build_launch_command(prepared, credential, runtime_options, options)
            .await?;
        self.launch(command)
    }

    pub async fn launch_instance(
        &self,
        instance_name: String,
        credential: &Credential,
        runtime_options: &RuntimeOptions,
        options: &LaunchOptions,
    ) -> Result<LaunchedInstance> {
        let instance =
            self.inspect_instance(instance_name.clone())
                .await?
                .ok_or(Error::InstanceNotFound {
                    name: instance_name,
                })?;
        let prepared = self.load_instance(instance).await?;
        self.launch_prepared_instance(&prepared, credential, runtime_options, options)
            .await
    }

    fn game_storage(&self) -> GameStorage {
        Storage::new(self.storage_root.clone(), self.root_layout.clone())
    }

    fn instance(&self, instance_name: String) -> Result<InstanceStorage> {
        Ok(self
            .game_storage()
            .instance(instance_name, self.instance_layout.clone())?)
    }

    fn resolve_driver(&self, descriptor: DriverDescriptor) -> Result<Arc<dyn Driver>> {
        self.drivers.driver(descriptor, &self.origins)
    }

    fn inspect_drivers(&self) -> Result<Vec<Arc<dyn Driver>>> {
        self.drivers
            .descriptors()
            .map(|descriptor| self.resolve_driver(descriptor))
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    async fn apply_requested_addons(
        &self,
        session: &DownloadSession,
        instance: &InstanceStorage,
        driver: DriverDescriptor,
        game_version: &MinecraftVersionId,
        runtime_options: &RuntimeOptions,
        prepared: &mut elemental_driver::families::version_json::PreparedVersionJsonInstance,
        requested_addons: Vec<RequestedAddon>,
        patch_mode: Option<PatchMode>,
    ) -> Result<(PatchMode, Vec<InstalledAddonRecord>)> {
        let mut state = load_addon_state(instance).await?;
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
            let record = self
                .prepare_addon_record(
                    session,
                    instance,
                    driver,
                    game_version,
                    runtime_options,
                    state.patch_mode,
                    requested,
                )
                .await?;
            state.upsert(record);
        }

        state.validate_modes()?;
        let trace = sync_instance_patches(instance, &state).await?;
        save_addon_state(instance, &state).await?;
        prepared.metadata = trace.metadata;
        Ok((state.patch_mode, state.addons))
    }

    #[allow(clippy::too_many_arguments)]
    async fn prepare_addon_record(
        &self,
        session: &DownloadSession,
        instance: &InstanceStorage,
        driver: DriverDescriptor,
        game_version: &MinecraftVersionId,
        runtime_options: &RuntimeOptions,
        instance_policy: PatchMode,
        requested: RequestedAddon,
    ) -> Result<InstalledAddonRecord> {
        let context = AddonContext {
            driver,
            game_version: Some(game_version),
            instance,
        };

        let (addon, version, addon_patch_mode) = match requested {
            RequestedAddon::BySpec(spec) => {
                let addon = self.addons.addon(&spec.id)?;
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
                session,
                instance,
                driver,
                game_version,
                addon_version: version.as_deref(),
                runtime_options,
                runtime_resolver: &self.runtime_resolver,
            })
            .await?;

        let managed_files = output
            .managed_files
            .iter()
            .map(|path| normalize_managed_file_path(&instance.path, path))
            .collect::<elemental_addon::Result<Vec<String>>>()?;

        Ok(InstalledAddonRecord {
            id: descriptor.id.into_owned(),
            name: descriptor.name.into_owned(),
            version: output.version.or(version),
            origin: AddonOrigin::Elemental,
            patch_mode: addon_patch_mode,
            patch_mode_support: support,
            requires_runtime_hook: addon.requires_runtime_hook(),
            managed_files,
            metadata_patch: output.metadata_patch,
        })
    }

    async fn prepare_launch_addons(
        &self,
        prepared: &PreparedInstance,
        runtime_options: &RuntimeOptions,
        options: &LaunchOptions,
    ) -> Result<(Vec<InstalledAddonRecord>, Option<PistonMetaData>)> {
        let pending_specs = options
            .addons
            .iter()
            .filter(|spec| {
                !prepared
                    .addons
                    .iter()
                    .any(|installed| installed.id == spec.id)
            })
            .fold(Vec::<AddonSpec>::new(), |mut acc, spec| {
                if !acc.iter().any(|existing| existing.id == spec.id) {
                    acc.push(spec.clone());
                }
                acc
            });
        if pending_specs.is_empty() {
            return Ok((Vec::new(), None));
        }

        let session = self
            .downloader
            .session(format!(
                "prepare launch addons {}",
                prepared.launch_version.metadata.id
            ))
            .open()
            .await?;

        let records_result: Result<Vec<InstalledAddonRecord>> = async {
            let mut records = Vec::with_capacity(pending_specs.len());
            for spec in pending_specs {
                let record = self
                    .prepare_addon_record(
                        &session,
                        &prepared.launch_version.version,
                        prepared.driver,
                        &prepared.game_version,
                        runtime_options,
                        PatchMode::Virtual,
                        RequestedAddon::BySpec(spec),
                    )
                    .await?;
                records.push(record);
            }
            Ok(records)
        }
        .await;

        let report = session.finish().await;
        let records = records_result?;
        report?.success()?;

        let patches = records
            .iter()
            .filter_map(|record| {
                record
                    .metadata_patch
                    .as_ref()
                    .map(|patch| NamedMetadataPatch {
                        id: record.id.clone(),
                        effective_mode: PatchMode::Virtual,
                        patch: patch.clone(),
                    })
            })
            .collect::<Vec<_>>();

        let metadata_override = if patches.is_empty() {
            None
        } else {
            Some(apply_metadata_patches(
                prepared.launch_version.metadata.clone(),
                &patches,
            )?)
        };

        Ok((records, metadata_override))
    }

    fn ensure_required_hooks_registered(&self, records: &[InstalledAddonRecord]) -> Result<()> {
        for record in records {
            if record.origin == AddonOrigin::Elemental
                && record.requires_runtime_hook
                && self.addons.get(&record.id).is_none()
            {
                return Err(Error::AddonNotRegistered {
                    id: record.id.clone(),
                });
            }
        }
        Ok(())
    }

    async fn apply_launch_addon_hooks(
        &self,
        prepared: &PreparedInstance,
        launch_records: &[InstalledAddonRecord],
        credential: &Credential,
        options: &mut LaunchOptions,
    ) -> Result<()> {
        for record in prepared.addons.iter().chain(launch_records.iter()) {
            if record.origin != AddonOrigin::Elemental {
                continue;
            }
            let Some(addon) = self.addons.get(&record.id) else {
                if record.requires_runtime_hook {
                    return Err(Error::AddonNotRegistered {
                        id: record.id.clone(),
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
                        record,
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

    async fn detect_instance_addons(
        &self,
        storage: &InstanceStorage,
    ) -> Result<Vec<InstalledAddon>> {
        let state = load_addon_state(storage).await?;
        let mut addons = state
            .addons
            .iter()
            .map(|record| record.to_installed(state.patch_mode))
            .collect::<Vec<_>>();

        if self.addons.iter().next().is_none() {
            return Ok(addons);
        }

        let probe = InstanceProbe::collect(storage).await?;
        for addon in self.addons.iter() {
            let descriptor = addon.descriptor();
            if addons.iter().any(|item| item.id == descriptor.id) {
                continue;
            }
            if let Some(detected) = addon.inspect(&probe).await? {
                addons.push(detected);
            }
        }
        Ok(addons)
    }
}

fn resolve_instance_game_version(
    layout: &InstanceStorage,
    known: Option<MinecraftVersionId>,
) -> Result<MinecraftVersionId> {
    if let Some(version) = known {
        return Ok(version);
    }
    let base = read_base_metadata(layout)?;
    let raw = base
        .inherits_from
        .or(base.client_version)
        .unwrap_or(base.id);
    Ok(MinecraftVersionId::from(raw))
}

pub(crate) enum RequestedAddon {
    BySpec(AddonSpec),
    Direct {
        addon: Arc<dyn Addon>,
        version: Option<String>,
        patch_mode: AddonPatchMode,
    },
}

/// Fluent builder for installing addons onto an existing instance via [`Launcher::install_addons`].
pub struct InstallAddonsBuilder<'a> {
    launcher: &'a Launcher,
    instance_name: String,
    runtime_options: Option<&'a RuntimeOptions>,
    session: Option<DownloadSession>,
    addons: Vec<RequestedAddon>,
    patch_mode: Option<PatchMode>,
}

impl<'a> InstallAddonsBuilder<'a> {
    pub fn runtime_options(mut self, options: &'a RuntimeOptions) -> Self {
        self.runtime_options = Some(options);
        self
    }

    pub fn session(mut self, session: DownloadSession) -> Self {
        self.session = Some(session);
        self
    }

    pub fn addon(mut self, spec: AddonSpec) -> Self {
        self.addons.push(RequestedAddon::BySpec(spec));
        self
    }

    pub fn with_addon(mut self, addon: impl Addon + 'static, version: Option<String>) -> Self {
        self.addons.push(RequestedAddon::Direct {
            addon: Arc::new(addon),
            version,
            patch_mode: AddonPatchMode::Inherit,
        });
        self
    }

    pub fn with_addon_mode(
        mut self,
        addon: impl Addon + 'static,
        version: Option<String>,
        patch_mode: AddonPatchMode,
    ) -> Self {
        self.addons.push(RequestedAddon::Direct {
            addon: Arc::new(addon),
            version,
            patch_mode,
        });
        self
    }

    pub fn patch_mode(mut self, mode: PatchMode) -> Self {
        self.patch_mode = Some(mode);
        self
    }

    pub async fn run(self) -> Result<PreparedInstance> {
        let session = match self.session {
            Some(session) => session,
            None => {
                self.launcher
                    .downloader
                    .session(format!("install addons {}", self.instance_name))
                    .open()
                    .await?
            }
        };
        let default_runtime = RuntimeOptions::default();
        let runtime_options = self.runtime_options.unwrap_or(&default_runtime);
        let prepared = self
            .launcher
            .install_addons_in_session(
                &session,
                self.instance_name,
                runtime_options,
                self.addons,
                self.patch_mode,
            )
            .await;
        let report = session.finish().await;
        let prepared = prepared?;
        let report = report?;
        report.success()?;
        Ok(prepared)
    }
}

/// Fluent builder for configuring and running [`Launcher::prepare_instance`].
pub struct PrepareInstanceBuilder<'a> {
    launcher: &'a Launcher,
    instance_name: String,
    driver: DriverSpec,
    runtime_options: Option<&'a RuntimeOptions>,
    session: Option<DownloadSession>,
    on_stage: Option<Box<dyn Fn(PrepareStage) + Send + Sync + 'a>>,
    addons: Vec<RequestedAddon>,
    patch_mode: Option<PatchMode>,
}

impl<'a> PrepareInstanceBuilder<'a> {
    pub fn runtime_options(mut self, options: &'a RuntimeOptions) -> Self {
        self.runtime_options = Some(options);
        self
    }

    pub fn session(mut self, session: DownloadSession) -> Self {
        self.session = Some(session);
        self
    }

    pub fn on_stage<F>(mut self, observer: F) -> Self
    where
        F: Fn(PrepareStage) + Send + Sync + 'a,
    {
        self.on_stage = Some(Box::new(observer));
        self
    }

    pub fn addon(mut self, spec: AddonSpec) -> Self {
        self.addons.push(RequestedAddon::BySpec(spec));
        self
    }

    pub fn with_addon(mut self, addon: impl Addon + 'static, version: Option<String>) -> Self {
        self.addons.push(RequestedAddon::Direct {
            addon: Arc::new(addon),
            version,
            patch_mode: AddonPatchMode::Inherit,
        });
        self
    }

    pub fn with_addon_mode(
        mut self,
        addon: impl Addon + 'static,
        version: Option<String>,
        patch_mode: AddonPatchMode,
    ) -> Self {
        self.addons.push(RequestedAddon::Direct {
            addon: Arc::new(addon),
            version,
            patch_mode,
        });
        self
    }

    pub fn patch_mode(mut self, mode: PatchMode) -> Self {
        self.patch_mode = Some(mode);
        self
    }

    pub async fn run(self) -> Result<PreparedInstance> {
        let session = match self.session {
            Some(session) => session,
            None => {
                self.launcher
                    .downloader
                    .session(format!("prepare {}", self.instance_name))
                    .open()
                    .await?
            }
        };
        let default_runtime = RuntimeOptions::default();
        let runtime_options = self.runtime_options.unwrap_or(&default_runtime);
        let prepared = self
            .launcher
            .prepare_in_session(
                &session,
                self.instance_name,
                self.driver,
                runtime_options,
                self.on_stage.as_deref(),
                self.addons,
                self.patch_mode,
            )
            .await;
        let report = session.finish().await;
        let prepared = prepared?;
        let report = report?;
        report.success()?;
        Ok(prepared)
    }
}

fn driver_refs(drivers: &[Arc<dyn Driver>]) -> Vec<&dyn Driver> {
    drivers.iter().map(Arc::as_ref).collect()
}

fn summarize_installed_instance(
    instance: InstalledInstance,
    addons: Vec<InstalledAddon>,
) -> Instance {
    let instance_name = instance.storage.display_name();

    Instance {
        instance_name,
        instance_root: instance.storage.path,
        driver: instance.driver,
        addons,
    }
}

#[cfg(test)]
mod tests {
    use elemental_driver::{
        descriptors::VANILLA_DRIVER,
        drivers::vanilla::{catalog::VanillaCatalog, driver::VanillaDriver},
    };

    use crate::DriverCatalog;

    use super::*;

    #[test]
    fn registration_rejects_builtin_and_duplicate_descriptors() {
        let custom = DriverDescriptor {
            id: "third-party",
            name: "Third Party",
        };
        let launcher = Launcher::builder()
            .build()
            .register_driver(DriverRegistration::new(
                custom,
                Arc::new(VanillaDriver::with_defaults()),
                DriverCatalog::game_only(custom, VanillaCatalog::with_defaults()),
            ))
            .unwrap();

        assert!(matches!(
            launcher.clone().register_driver(DriverRegistration::new(
                custom,
                Arc::new(VanillaDriver::with_defaults()),
                DriverCatalog::game_only(custom, VanillaCatalog::with_defaults()),
            ),),
            Err(Error::DriverAlreadyRegistered { .. })
        ));
        assert!(matches!(
            launcher.register_driver(DriverRegistration::new(
                VANILLA_DRIVER,
                Arc::new(VanillaDriver::with_defaults()),
                DriverCatalog::game_only(VANILLA_DRIVER, VanillaCatalog::with_defaults()),
            ),),
            Err(Error::DriverAlreadyRegistered { .. })
        ));
    }
}
