use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use elemental_addon::{PatchMode, load_addon_state, sync_instance_patches};
use elemental_core::{
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
        GameStorage, InstanceStorage, PreparedVersionJsonInstance, SharedInstanceLayout,
        SharedRootLayout, VersionJsonGameStorageExt, read_base_metadata,
    },
};
use elemental_infra::downloader::DownloadSession;

use crate::{
    addon::AddonRegistry,
    error::{Error, Result},
    origin::DriverOrigins,
    registration::DriverRegistry,
    request::RuntimeOptions,
    result::{
        InspectReport, Instance, InstanceInspectionFailure, LaunchedInstance, PreparedInstance,
        RemoveInstanceOutcome,
    },
    spec::DriverSpec,
};

use super::addons::{
    AddonPreparationContext, RequestedAddon, apply_requested_addons, detect_instance_addons,
    ensure_required_hooks_registered,
};

pub(super) struct PrepareSessionParams<'a> {
    instance_name: String,
    driver: DriverSpec,
    runtime_options: &'a RuntimeOptions,
    on_stage: Option<&'a (dyn Fn(PrepareStage) + Send + Sync)>,
    requested_addons: Vec<RequestedAddon>,
    patch_mode: Option<PatchMode>,
}

impl<'a> PrepareSessionParams<'a> {
    pub(super) fn new(
        instance_name: String,
        driver: DriverSpec,
        runtime_options: &'a RuntimeOptions,
        on_stage: Option<&'a (dyn Fn(PrepareStage) + Send + Sync)>,
        requested_addons: Vec<RequestedAddon>,
        patch_mode: Option<PatchMode>,
    ) -> Self {
        Self {
            instance_name,
            driver,
            runtime_options,
            on_stage,
            requested_addons,
            patch_mode,
        }
    }
}

pub(super) struct LauncherServices<'a> {
    storage_root: &'a Path,
    root_layout: &'a SharedRootLayout,
    instance_layout: &'a SharedInstanceLayout,
    origins: &'a DriverOrigins,
    runtime_resolver: &'a RuntimeResolver,
    drivers: &'a DriverRegistry,
    addons: &'a AddonRegistry,
}

impl<'a> LauncherServices<'a> {
    pub(super) fn new(
        storage_root: &'a Path,
        root_layout: &'a SharedRootLayout,
        instance_layout: &'a SharedInstanceLayout,
        origins: &'a DriverOrigins,
        runtime_resolver: &'a RuntimeResolver,
        drivers: &'a DriverRegistry,
        addons: &'a AddonRegistry,
    ) -> Self {
        Self {
            storage_root,
            root_layout,
            instance_layout,
            origins,
            runtime_resolver,
            drivers,
            addons,
        }
    }

    pub(super) fn game_storage(&self) -> GameStorage {
        Storage::new(self.storage_root, self.root_layout.clone())
    }

    pub(super) fn addons(&self) -> &'a AddonRegistry {
        self.addons
    }

    pub(super) fn runtime_resolver(&self) -> &'a RuntimeResolver {
        self.runtime_resolver
    }

    pub(super) fn instance(&self, instance_name: String) -> Result<InstanceStorage> {
        Ok(self
            .game_storage()
            .instance(instance_name, self.instance_layout.clone())?)
    }

    pub(super) async fn inspect_instances(&self) -> Result<InspectReport> {
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
                    let addons = detect_instance_addons(self.addons, &instance.storage).await?;
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

    pub(super) async fn inspect_instance(&self, instance_name: String) -> Result<Option<Instance>> {
        let instance = self.instance(instance_name)?;
        let drivers = self.inspect_drivers()?;
        let driver_refs = driver_refs(&drivers);

        match InstalledInstance::detect(instance, &driver_refs).await? {
            Some(installed) => {
                let addons = detect_instance_addons(self.addons, &installed.storage).await?;
                Ok(Some(summarize_installed_instance(installed, addons)))
            }
            None => Ok(None),
        }
    }

    pub(super) async fn remove_instance(
        &self,
        instance_name: String,
    ) -> Result<RemoveInstanceOutcome> {
        let path = self.game_storage().instance_root(instance_name)?;
        match tokio::fs::remove_dir_all(&path).await {
            Ok(()) => Ok(RemoveInstanceOutcome::Removed),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                Ok(RemoveInstanceOutcome::NotFound)
            }
            Err(source) => Err(source.into()),
        }
    }

    pub(super) async fn prepare_in_session(
        &self,
        session: &DownloadSession,
        params: PrepareSessionParams<'_>,
    ) -> Result<PreparedInstance> {
        let instance = self
            .game_storage()
            .ensure_instance(params.instance_name, self.instance_layout.clone())
            .await?;
        let resolved_driver = self.resolve_driver(params.driver.descriptor())?;
        let prepared = resolved_driver
            .prepare(&PrepareRequest {
                session,
                instance: &instance,
                game_version: params.driver.game_version(),
                loader_version: params.driver.loader_version(),
                runtime_options: params.runtime_options,
                runtime_resolver: self.runtime_resolver,
                on_stage: params.on_stage,
            })
            .await?;
        let ctx = AddonPreparationContext {
            session,
            instance: &instance,
            driver: params.driver.descriptor(),
            game_version: params.driver.game_version(),
            runtime_options: params.runtime_options,
            runtime_resolver: self.runtime_resolver,
        };
        self.apply_addons_and_wrap(&ctx, prepared, params.requested_addons, params.patch_mode)
            .await
    }

    pub(super) async fn install_addons_in_session(
        &self,
        session: &DownloadSession,
        instance_name: String,
        runtime_options: &RuntimeOptions,
        requested_addons: Vec<RequestedAddon>,
        patch_mode: Option<PatchMode>,
    ) -> Result<PreparedInstance> {
        let layout = self.instance(instance_name.clone())?;
        let drivers = self.inspect_drivers()?;
        let driver_refs = driver_refs(&drivers);
        let installed = InstalledInstance::detect(layout, &driver_refs)
            .await?
            .ok_or(Error::InstanceNotFound {
                name: instance_name,
            })?;
        let layout = installed.storage;
        let driver_descriptor = installed.driver.driver;
        let prepared = self
            .resolve_driver(driver_descriptor)?
            .load_prepared(&layout)
            .await?;
        let game_version = resolve_instance_game_version(&layout, installed.driver.game_version)?;

        let ctx = AddonPreparationContext {
            session,
            instance: &layout,
            driver: driver_descriptor,
            game_version: &game_version,
            runtime_options,
            runtime_resolver: self.runtime_resolver,
        };
        self.apply_addons_and_wrap(&ctx, prepared, requested_addons, patch_mode)
            .await
    }

    async fn apply_addons_and_wrap(
        &self,
        ctx: &AddonPreparationContext<'_>,
        mut prepared: PreparedVersionJsonInstance,
        requested_addons: Vec<RequestedAddon>,
        patch_mode: Option<PatchMode>,
    ) -> Result<PreparedInstance> {
        let (effective_policy, addons) = apply_requested_addons(
            self.addons,
            ctx,
            &mut prepared,
            requested_addons,
            patch_mode,
        )
        .await?;
        Ok(PreparedInstance::new(
            ctx.driver,
            ctx.game_version.clone(),
            prepared,
            effective_policy,
            addons,
        ))
    }

    pub(super) async fn load_instance(&self, instance: Instance) -> Result<PreparedInstance> {
        let layout = self.instance(instance.instance_name)?;
        let driver_spec = instance.driver.driver;
        let game_version = resolve_instance_game_version(&layout, instance.driver.game_version)?;
        let mut prepared = self
            .resolve_driver(driver_spec)?
            .load_prepared(&layout)
            .await?;
        let state = load_addon_state(&layout).await?;
        ensure_required_hooks_registered(self.addons, &state.addons)?;
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

    fn resolve_driver(&self, descriptor: DriverDescriptor) -> Result<Arc<dyn Driver>> {
        self.drivers.driver(descriptor, self.origins)
    }

    fn inspect_drivers(&self) -> Result<Vec<Arc<dyn Driver>>> {
        self.drivers
            .descriptors()
            .map(|descriptor| self.resolve_driver(descriptor))
            .collect()
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

fn driver_refs(drivers: &[Arc<dyn Driver>]) -> Vec<&dyn Driver> {
    drivers.iter().map(Arc::as_ref).collect()
}

fn summarize_installed_instance(
    instance: InstalledInstance,
    addons: Vec<elemental_addon::InstalledAddon>,
) -> Instance {
    let instance_name = instance.storage.display_name();

    Instance {
        instance_name,
        instance_root: PathBuf::from(&instance.storage.path),
        driver: instance.driver,
        addons,
    }
}

pub(super) fn spawn_launch_command(command: LaunchCommand) -> Result<LaunchedInstance> {
    let runtime = command.runtime.clone();
    let child = process::spawn_command(command)?;
    Ok(LaunchedInstance { runtime, child })
}
