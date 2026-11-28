use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use elemental_addon::{
    Addon, AddonModify, AddonPatchMode, InstalledAddon, PatchMode, PatchTrace,
    remove_addon_from_instance, reorder_instance_addons, set_addon_patch_mode,
    set_instance_patch_mode, trace_instance_patches,
};
use elemental_core::{
    auth::credential::Credential, http::SwappableClient, launcher::command::LaunchCommand,
    runtime::RuntimeResolver,
};
use elemental_driver::families::version_json::{SharedInstanceLayout, SharedRootLayout};
use elemental_infra::downloader::ElementalDownloader;

use crate::{
    addon::AddonRegistry,
    error::{Error, Result},
    origin::DriverOrigins,
    registration::{DriverRegistration, DriverRegistry},
    request::{LaunchOptions, RuntimeOptions},
    result::{InspectReport, Instance, LaunchedInstance, PreparedInstance, RemoveInstanceOutcome},
    spec::DriverSpec,
};

mod addons;
mod ops;
mod scope;

pub use scope::ActionScope;

use addons::detect_instance_addons;
use ops::LauncherServices;

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
        self.services().inspect_instances().await
    }

    pub async fn inspect_instance(&self, instance_name: String) -> Result<Option<Instance>> {
        self.services().inspect_instance(instance_name).await
    }

    /// Removes an installed instance and its root directory.
    ///
    /// This does not require the instance metadata to be valid, so corrupted
    /// installations can also be cleaned up.
    pub async fn remove_instance(
        &self,
        instance_name: impl Into<String>,
    ) -> Result<RemoveInstanceOutcome> {
        self.services().remove_instance(instance_name.into()).await
    }

    /// Starts a scoped execution context for preparing instances, installing addons, or launching.
    pub fn scope(&self) -> ActionScope<'_> {
        ActionScope::new(self.services(), &self.downloader)
    }

    /// Install or refresh an instance with a default download session.
    pub async fn prepare(
        &self,
        instance_name: String,
        driver: DriverSpec,
        runtime_options: &RuntimeOptions,
    ) -> Result<PreparedInstance> {
        self.scope()
            .runtime_options(runtime_options)
            .prepare_instance(instance_name, driver)
            .await
    }

    pub async fn load_instance(&self, instance: Instance) -> Result<PreparedInstance> {
        self.services().load_instance(instance).await
    }

    /// Returns all addons installed or detected on the named instance.
    pub async fn instance_addons(
        &self,
        instance_name: impl Into<String>,
    ) -> Result<Vec<InstalledAddon>> {
        let layout = self.services().instance(instance_name.into())?;
        detect_instance_addons(&self.addons, &layout).await
    }

    pub async fn remove_addon(
        &self,
        instance_name: impl Into<String>,
        addon_id: &str,
    ) -> Result<bool> {
        let layout = self.services().instance(instance_name.into())?;
        Ok(remove_addon_from_instance(&layout, addon_id).await?)
    }

    pub async fn set_instance_patch_mode(
        &self,
        instance_name: impl Into<String>,
        mode: PatchMode,
    ) -> Result<PatchTrace> {
        let layout = self.services().instance(instance_name.into())?;
        Ok(set_instance_patch_mode(&layout, mode).await?)
    }

    pub async fn set_addon_patch_mode(
        &self,
        instance_name: impl Into<String>,
        addon_id: &str,
        mode: AddonPatchMode,
    ) -> Result<PatchTrace> {
        let layout = self.services().instance(instance_name.into())?;
        Ok(set_addon_patch_mode(&layout, addon_id, mode).await?)
    }

    pub async fn reorder_addons(
        &self,
        instance_name: impl Into<String>,
        ordered_ids: &[&str],
    ) -> Result<PatchTrace> {
        let layout = self.services().instance(instance_name.into())?;
        Ok(reorder_instance_addons(&layout, ordered_ids).await?)
    }

    pub async fn trace_instance_patches(
        &self,
        instance_name: impl Into<String>,
    ) -> Result<PatchTrace> {
        let layout = self.services().instance(instance_name.into())?;
        Ok(trace_instance_patches(&layout).await?)
    }

    /// Loads an in-memory [`AddonModify`] draft for previewing addon order and patch-mode
    /// changes before committing them to disk.
    pub async fn modify_instance_addons(
        &self,
        instance_name: impl Into<String>,
    ) -> Result<AddonModify> {
        let layout = self.services().instance(instance_name.into())?;
        Ok(AddonModify::load(layout).await?)
    }

    pub async fn build_launch_command(
        &self,
        prepared: &PreparedInstance,
        credential: &Credential,
        runtime_options: &RuntimeOptions,
        options: &LaunchOptions,
    ) -> Result<LaunchCommand> {
        self.scope()
            .runtime_options(runtime_options)
            .build_launch_command(prepared, credential, options)
            .await
    }

    pub fn launch(&self, command: LaunchCommand) -> Result<LaunchedInstance> {
        ops::spawn_launch_command(command)
    }

    pub async fn launch_prepared_instance(
        &self,
        prepared: &PreparedInstance,
        credential: &Credential,
        runtime_options: &RuntimeOptions,
        options: &LaunchOptions,
    ) -> Result<LaunchedInstance> {
        self.scope()
            .runtime_options(runtime_options)
            .launch_prepared(prepared, credential, options)
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

    fn services(&self) -> LauncherServices<'_> {
        LauncherServices::new(
            &self.storage_root,
            &self.root_layout,
            &self.instance_layout,
            &self.origins,
            &self.runtime_resolver,
            &self.drivers,
            &self.addons,
        )
    }
}

#[cfg(test)]
mod tests {
    use elemental_driver::{
        descriptors::{DriverDescriptor, VANILLA_DRIVER},
        drivers::vanilla::{catalog::VanillaCatalog, driver::VanillaDriver},
    };

    use crate::DriverCatalog;

    use super::*;

    fn custom_registration(descriptor: DriverDescriptor) -> DriverRegistration {
        DriverRegistration::new(
            descriptor,
            Arc::new(VanillaDriver::with_defaults()),
            DriverCatalog::game_only(descriptor, VanillaCatalog::with_defaults()),
        )
    }

    #[test]
    fn registration_rejects_builtin_and_duplicate_descriptors() {
        let custom = DriverDescriptor {
            id: "third-party",
            name: "Third Party",
        };
        let launcher = Launcher::builder()
            .build()
            .register_driver(custom_registration(custom))
            .unwrap();

        assert!(matches!(
            launcher
                .clone()
                .register_driver(custom_registration(custom)),
            Err(Error::DriverAlreadyRegistered { .. })
        ));
        assert!(matches!(
            launcher.register_driver(custom_registration(VANILLA_DRIVER)),
            Err(Error::DriverAlreadyRegistered { .. })
        ));
    }
}
