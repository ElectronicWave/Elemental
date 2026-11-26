#![cfg_attr(not(feature = "driver-all"), allow(dead_code, unreachable_code))]
#![allow(unreachable_patterns)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use elemental_core::{
    auth::credential::Credential,
    http::SwappableClient,
    launcher::{command::LaunchCommand, process},
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
        VersionJsonGameStorageExt,
    },
};
use elemental_infra::downloader::{DownloadSession, ElementalDownloader};

use crate::{
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

#[cfg(feature = "yggdrasil-auth")]
use crate::authlib_injector::{self, AuthlibInjector};

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
        Self {
            storage_root,
            http_client,
            downloader,
            root_layout,
            instance_layout,
            origins,
            runtime_resolver,
            drivers,
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

    pub fn drivers(&self) -> &DriverRegistry {
        &self.drivers
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
                Ok(instance) => instances.push(summarize_installed_instance(instance)),
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

        Ok(InstalledInstance::detect(instance, &driver_refs)
            .await?
            .map(summarize_installed_instance))
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
    pub(crate) async fn prepare_in_session(
        &self,
        session: &DownloadSession,
        instance_name: String,
        driver: DriverSpec,
        runtime_options: &RuntimeOptions,
        on_stage: Option<&(dyn Fn(PrepareStage) + Send + Sync)>,
    ) -> Result<PreparedInstance> {
        let instance = self
            .game_storage()
            .ensure_instance(instance_name, self.instance_layout.clone())
            .await?;
        let resolved_driver = self.resolve_driver(driver.descriptor())?;
        let prepared = resolved_driver
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
        Ok(PreparedInstance::new(driver.descriptor(), prepared))
    }

    pub async fn load_instance(&self, instance: Instance) -> Result<PreparedInstance> {
        let layout = self.instance(instance.instance_name)?;
        let driver_spec = instance.driver.driver;
        let prepared = self
            .resolve_driver(driver_spec)?
            .load_prepared(&layout)
            .await?;
        Ok(PreparedInstance::new(driver_spec, prepared))
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
        #[cfg(feature = "yggdrasil-auth")]
        let options = {
            let mut options = options.clone();
            self.inject_authlib_injector(credential, &mut options)
                .await?;
            options
        };
        #[cfg(not(feature = "yggdrasil-auth"))]
        let options = options.clone();
        prepared.build_launch_command(credential, runtime, &options)
    }

    #[cfg(feature = "yggdrasil-auth")]
    pub async fn install_authlib_injector(&self) -> Result<AuthlibInjector> {
        authlib_injector::install(&self.storage_root, &self.downloader).await
    }

    #[cfg(feature = "yggdrasil-auth")]
    async fn inject_authlib_injector(
        &self,
        credential: &Credential,
        options: &mut LaunchOptions,
    ) -> Result<()> {
        let Some(auth_server) = &credential.auth_server else {
            return Ok(());
        };
        let injector = self.install_authlib_injector().await?;
        options
            .extra_jvm_arguments
            .extend(injector.jvm_arguments(auth_server));
        Ok(())
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
}

/// Fluent builder for configuring and running [`Launcher::prepare_instance`].
pub struct PrepareInstanceBuilder<'a> {
    launcher: &'a Launcher,
    instance_name: String,
    driver: DriverSpec,
    runtime_options: Option<&'a RuntimeOptions>,
    session: Option<DownloadSession>,
    on_stage: Option<Box<dyn Fn(PrepareStage) + Send + Sync + 'a>>,
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

fn summarize_installed_instance(instance: InstalledInstance) -> Instance {
    let instance_name = instance.storage.display_name();

    Instance {
        instance_name,
        instance_root: instance.storage.path,
        driver: instance.driver,
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
