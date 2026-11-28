use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use elemental_addon::{
    Addon, AddonModify, AddonPatchMode, InstalledAddon, PatchMode, PatchTrace, load_addon_state,
    remove_addon_from_instance, reorder_instance_addons, set_addon_patch_mode,
    set_instance_patch_mode, sync_instance_patches, trace_instance_patches,
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
        Driver, DriverDescriptor, InstalledInstance, PrepareRequest, PrepareStage,
        inspect_instances,
    },
    families::version_json::{
        GameStorage, InstanceStorage, SharedInstanceLayout, SharedRootLayout,
        VersionJsonGameStorageExt, read_base_metadata,
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

mod addons;

use addons::{
    AddonPreparationContext, RequestedAddon, apply_launch_addon_hooks, apply_requested_addons,
    detect_instance_addons, ensure_required_hooks_registered, order_and_trace_launch_entries,
    resolve_launch_addons,
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
        let addons = AddonRegistry::new(http_client.clone(), &origins);
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
                    let addons = detect_instance_addons(&self.addons, &instance.storage).await?;
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
                let addons = detect_instance_addons(&self.addons, &installed.storage).await?;
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

    async fn prepare_in_session(
        &self,
        session: &DownloadSession,
        request: PrepareInstanceBuilder<'_>,
    ) -> Result<PreparedInstance> {
        let default_runtime = RuntimeOptions::default();
        let runtime_options = request.runtime_options.unwrap_or(&default_runtime);
        let instance = self
            .game_storage()
            .ensure_instance(request.instance_name, self.instance_layout.clone())
            .await?;
        let resolved_driver = self.resolve_driver(request.driver.descriptor())?;
        let mut prepared = resolved_driver
            .prepare(&PrepareRequest {
                session,
                instance: &instance,
                game_version: request.driver.game_version(),
                loader_version: request.driver.loader_version(),
                runtime_options,
                runtime_resolver: &self.runtime_resolver,
                on_stage: request.on_stage.as_deref(),
            })
            .await?;
        let ctx = AddonPreparationContext {
            session,
            instance: &instance,
            driver: request.driver.descriptor(),
            game_version: request.driver.game_version(),
            runtime_options,
            runtime_resolver: &self.runtime_resolver,
        };
        let (effective_policy, addons) = apply_requested_addons(
            &self.addons,
            &ctx,
            &mut prepared,
            request.addons,
            request.patch_mode,
        )
        .await?;
        Ok(PreparedInstance::new(
            request.driver.descriptor(),
            request.driver.game_version().clone(),
            prepared,
            effective_policy,
            addons,
        ))
    }

    async fn install_addons_in_session(
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

        let ctx = AddonPreparationContext {
            session,
            instance: &layout,
            driver: driver_descriptor,
            game_version: &game_version,
            runtime_options,
            runtime_resolver: &self.runtime_resolver,
        };
        let (effective_policy, addons) = apply_requested_addons(
            &self.addons,
            &ctx,
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
        ensure_required_hooks_registered(&self.addons, &state.addons)?;
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
        detect_instance_addons(&self.addons, &layout).await
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

    /// Loads an in-memory [`AddonModify`] draft for previewing addon order and patch-mode
    /// changes before committing them to disk.
    pub async fn modify_instance_addons(
        &self,
        instance_name: impl Into<String>,
    ) -> Result<AddonModify> {
        let layout = self.instance(instance_name.into())?;
        Ok(AddonModify::load(layout).await?)
    }

    /// Starts configuring a launch for a [`PreparedInstance`] and player [`Credential`].
    pub fn launch_prepared<'a>(
        &'a self,
        prepared: &'a PreparedInstance,
        credential: &'a Credential,
    ) -> LaunchBuilder<'a> {
        LaunchBuilder {
            launcher: self,
            prepared,
            credential,
            runtime_options: None,
            options: None,
            session: None,
            addons: Vec::new(),
        }
    }

    pub async fn build_launch_command(
        &self,
        prepared: &PreparedInstance,
        credential: &Credential,
        runtime_options: &RuntimeOptions,
        options: &LaunchOptions,
    ) -> Result<LaunchCommand> {
        self.launch_prepared(prepared, credential)
            .runtime_options(runtime_options)
            .options(options)
            .build_command()
            .await
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
        self.launch_prepared(prepared, credential)
            .runtime_options(runtime_options)
            .options(options)
            .run()
            .await
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
        self.addons.push(RequestedAddon::direct(
            addon,
            version,
            AddonPatchMode::Inherit,
        ));
        self
    }

    pub fn with_addon_mode(
        mut self,
        addon: impl Addon + 'static,
        version: Option<String>,
        patch_mode: AddonPatchMode,
    ) -> Self {
        self.addons
            .push(RequestedAddon::direct(addon, version, patch_mode));
        self
    }

    pub fn patch_mode(mut self, mode: PatchMode) -> Self {
        self.patch_mode = Some(mode);
        self
    }

    pub async fn run(self) -> Result<PreparedInstance> {
        let session = open_or_reuse_session(
            &self.launcher.downloader,
            self.session,
            &format!("install addons {}", self.instance_name),
        )
        .await?;
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
        finish_session_result(session, prepared).await
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
        self.addons.push(RequestedAddon::direct(
            addon,
            version,
            AddonPatchMode::Inherit,
        ));
        self
    }

    pub fn with_addon_mode(
        mut self,
        addon: impl Addon + 'static,
        version: Option<String>,
        patch_mode: AddonPatchMode,
    ) -> Self {
        self.addons
            .push(RequestedAddon::direct(addon, version, patch_mode));
        self
    }

    pub fn patch_mode(mut self, mode: PatchMode) -> Self {
        self.patch_mode = Some(mode);
        self
    }

    pub async fn run(mut self) -> Result<PreparedInstance> {
        let launcher = self.launcher;
        let session = open_or_reuse_session(
            &launcher.downloader,
            self.session.take(),
            &format!("prepare {}", self.instance_name),
        )
        .await?;
        let prepared = launcher.prepare_in_session(&session, self).await;
        finish_session_result(session, prepared).await
    }
}

/// Fluent builder for configuring and launching a [`PreparedInstance`] with optional launch-scoped addons.
pub struct LaunchBuilder<'a> {
    launcher: &'a Launcher,
    prepared: &'a PreparedInstance,
    credential: &'a Credential,
    runtime_options: Option<&'a RuntimeOptions>,
    options: Option<&'a LaunchOptions>,
    session: Option<DownloadSession>,
    addons: Vec<RequestedAddon>,
}

impl<'a> LaunchBuilder<'a> {
    pub fn runtime_options(mut self, options: &'a RuntimeOptions) -> Self {
        self.runtime_options = Some(options);
        self
    }

    pub fn options(mut self, options: &'a LaunchOptions) -> Self {
        self.options = Some(options);
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
        self.addons.push(RequestedAddon::direct(
            addon,
            version,
            AddonPatchMode::Inherit,
        ));
        self
    }

    pub fn with_addon_mode(
        mut self,
        addon: impl Addon + 'static,
        version: Option<String>,
        patch_mode: AddonPatchMode,
    ) -> Self {
        self.addons
            .push(RequestedAddon::direct(addon, version, patch_mode));
        self
    }

    /// Resolves all active addons (persistent + launch-scoped) from the clean
    /// baseline and returns the unified [`PatchTrace`].
    pub async fn trace(self) -> Result<PatchTrace> {
        let default_runtime = RuntimeOptions::default();
        let runtime_options = self.runtime_options.unwrap_or(&default_runtime);
        let (mut entries, maybe_trace) = resolve_launch_addons(
            &self.launcher.addons,
            &self.launcher.downloader,
            &self.launcher.runtime_resolver,
            self.prepared,
            runtime_options,
            self.session,
            self.addons,
        )
        .await?;
        match maybe_trace {
            Some(trace) => Ok(trace),
            None => order_and_trace_launch_entries(self.prepared, &mut entries),
        }
    }

    /// Builds the [`LaunchCommand`] with unified persistent and launch-scoped addons applied.
    pub async fn build_command(self) -> Result<LaunchCommand> {
        let default_runtime = RuntimeOptions::default();
        let runtime_options = self.runtime_options.unwrap_or(&default_runtime);
        let default_options = LaunchOptions::default();
        let mut launch_options = self.options.unwrap_or(&default_options).clone();

        let runtime = self
            .prepared
            .resolve_runtime(runtime_options, &self.launcher.runtime_resolver)
            .await?;
        let (entries, trace) = resolve_launch_addons(
            &self.launcher.addons,
            &self.launcher.downloader,
            &self.launcher.runtime_resolver,
            self.prepared,
            runtime_options,
            self.session,
            self.addons,
        )
        .await?;
        apply_launch_addon_hooks(
            self.prepared,
            &entries,
            self.credential,
            &mut launch_options,
        )
        .await?;
        self.prepared.build_launch_command(
            self.credential,
            runtime,
            &launch_options,
            trace.map(|t| t.metadata),
        )
    }

    /// Builds the launch command and spawns the game process.
    pub async fn run(self) -> Result<LaunchedInstance> {
        let launcher = self.launcher;
        let command = self.build_command().await?;
        launcher.launch(command)
    }
}

async fn open_or_reuse_session(
    downloader: &ElementalDownloader,
    session: Option<DownloadSession>,
    label: &str,
) -> Result<DownloadSession> {
    match session {
        Some(session) => Ok(session),
        None => Ok(downloader.session(label).open().await?),
    }
}

async fn finish_session_result(
    session: DownloadSession,
    prepared: Result<PreparedInstance>,
) -> Result<PreparedInstance> {
    let report = session.finish().await;
    let prepared = prepared?;
    let report = report?;
    report.success()?;
    Ok(prepared)
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
