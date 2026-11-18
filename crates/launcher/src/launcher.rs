#![cfg_attr(not(feature = "driver-all"), allow(dead_code, unreachable_code))]
#![allow(unreachable_patterns)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use elemental_core::{
    auth::credential::Credential,
    launcher::{command::LaunchCommand, process},
    runtime::{RuntimeResolver, distribution::Distribution},
    storage::Storage,
};
use elemental_driver::{
    driver::{Driver, DriverDescriptor, PrepareRequest},
    families::version_json::{
        BaseInstanceLayout, BaseRootLayout, VersionJsonGameStorageExt, VersionJsonInstanceLayout,
        VersionJsonRootLayout, builder::VersionJsonLaunchBuilder, inspect_instances,
        variables::LauncherVariables,
    },
    inspect::InstalledInstance,
    launch::resolve_prepared_version_runtime,
};
use elemental_infra::downloader::{DownloadSession, ElementalDownloader};

use crate::{
    builder::LauncherBuilder,
    error::{Error, Result},
    origin::DriverOrigins,
    registration::{
        DriverRegistration, builtin_registration, builtin_registrations, builtin_unsupported,
    },
    request::{LaunchOptions, RuntimeOptions},
    result::{
        InspectReport, Instance, InstanceInspectionFailure, LaunchedInstance, PreparedInstance,
        RemoveInstanceOutcome,
    },
    spec::DriverSpec,
};

#[cfg(feature = "yggdrasil-auth")]
use crate::authlib_injector::{self, AuthlibInjector};

type LauncherGameStorage<L> = Storage<L>;
type LauncherInstanceStorage<L, VL> = Storage<VL, LauncherGameStorage<L>>;

#[derive(Debug, Clone)]
pub struct Launcher<L = BaseRootLayout, VL = BaseInstanceLayout>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    storage_root: PathBuf,
    downloader: Arc<ElementalDownloader>,
    root_layout: L,
    instance_layout: VL,
    origins: DriverOrigins,
    runtime_resolver: RuntimeResolver,
    custom_drivers: Vec<DriverRegistration<L, VL>>,
}

impl Launcher<BaseRootLayout, BaseInstanceLayout> {
    pub fn builder() -> LauncherBuilder<BaseRootLayout, BaseInstanceLayout> {
        LauncherBuilder::new()
    }
}

impl<L, VL> Launcher<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone + Send,
{
    pub(crate) fn new(
        storage_root: PathBuf,
        downloader: Arc<ElementalDownloader>,
        root_layout: L,
        instance_layout: VL,
        origins: DriverOrigins,
        runtime_resolver: RuntimeResolver,
    ) -> Self {
        Self {
            storage_root,
            downloader,
            root_layout,
            instance_layout,
            origins,
            runtime_resolver,
            custom_drivers: Vec::new(),
        }
    }

    /// Adds a third-party driver after the launcher has been built.
    ///
    /// Built-in descriptors cannot be replaced. Use [`DriverSpec::custom`] to
    /// prepare an instance for a registered third-party driver.
    pub fn register_driver(mut self, registration: DriverRegistration<L, VL>) -> Result<Self> {
        let descriptor = registration.descriptor();
        if builtin_registration(descriptor).is_some()
            || self
                .custom_drivers
                .iter()
                .any(|registered| registered.descriptor() == descriptor)
        {
            return Err(Error::DriverAlreadyRegistered {
                id: descriptor.id.to_string(),
            });
        }

        self.custom_drivers.push(registration);
        Ok(self)
    }

    pub fn storage_root(&self) -> &Path {
        &self.storage_root
    }

    pub fn downloader(&self) -> &Arc<ElementalDownloader> {
        &self.downloader
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

    /// Install or refresh an instance with a default download session.
    pub async fn prepare(
        &self,
        instance_name: String,
        driver: DriverSpec,
        runtime_options: &RuntimeOptions,
    ) -> Result<PreparedInstance<L, VL>> {
        let session = self
            .downloader
            .session(format!("prepare {instance_name}"))
            .open()
            .await?;
        self.prepare_with_session(session, instance_name, driver, runtime_options)
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
    ) -> Result<PreparedInstance<L, VL>> {
        let prepared = self
            .prepare_in_session(&session, instance_name, driver, runtime_options)
            .await;
        let report = session.finish().await;
        let prepared = prepared?;
        let report = report?;
        report.success()?;
        Ok(prepared)
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
    ) -> Result<PreparedInstance<L, VL>> {
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
            })
            .await?;
        Ok(PreparedInstance::new(driver.descriptor(), prepared))
    }

    pub async fn load_instance(&self, instance: Instance) -> Result<PreparedInstance<L, VL>> {
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
        prepared: &PreparedInstance<L, VL>,
        credential: &Credential,
        runtime_options: &RuntimeOptions,
        options: &LaunchOptions,
    ) -> Result<LaunchCommand> {
        let runtime = resolve_prepared_version_runtime(
            &prepared.launch_version,
            runtime_options,
            &self.runtime_resolver,
        )
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
        let builder = launch_builder(credential, runtime, prepared, &options);
        Ok(builder.build_command()?)
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
        prepared: &PreparedInstance<L, VL>,
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

    fn game_storage(&self) -> LauncherGameStorage<L> {
        Storage::new(self.storage_root.clone(), self.root_layout.clone())
    }

    fn instance(&self, instance_name: String) -> Result<LauncherInstanceStorage<L, VL>> {
        Ok(self
            .game_storage()
            .instance(instance_name, self.instance_layout.clone())?)
    }

    fn resolve_driver(&self, descriptor: DriverDescriptor) -> Result<Arc<dyn Driver<L, VL>>> {
        if let Some(registration) = self
            .custom_drivers
            .iter()
            .find(|registration| registration.descriptor() == descriptor)
        {
            return Ok(registration.driver());
        }

        let registration =
            builtin_registration(descriptor).ok_or_else(|| builtin_unsupported(descriptor))?;
        Ok(Arc::from(
            registration.driver.build(descriptor, &self.origins)?,
        ))
    }
}

fn launch_builder<L, VL>(
    credential: &Credential,
    runtime: Distribution,
    prepared: &PreparedInstance<L, VL>,
    options: &LaunchOptions,
) -> VersionJsonLaunchBuilder<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let mut variables = LauncherVariables::default();

    if let Some(client_id) = &options.client_id {
        variables.clientid = client_id.clone();
    }

    if let Some(resolution) = &options.resolution {
        variables.resolution_width = resolution.width.clone();
        variables.resolution_height = resolution.height.clone();
    }

    if let (Some(name), Some(version)) = (&options.launcher_name, &options.launcher_version) {
        variables.launcher_name = name.clone();
        variables.launcher_version = version.clone();
    }

    if let Some(quick_play) = &options.quick_play {
        variables.quick_play_path = quick_play.path.clone();
        variables.quick_play_multiplayer = quick_play.multiplayer.clone();
        variables.quick_play_singleplayer = quick_play.singleplayer.clone();
        variables.quick_play_realms = quick_play.realms.clone();
    }

    VersionJsonLaunchBuilder::new(
        credential.clone(),
        runtime,
        prepared.launch_version.version.clone(),
        variables,
        options.extra_jvm_arguments.clone(),
        options.extra_game_arguments.clone(),
    )
}

impl<L, VL> Launcher<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone + Send,
{
    fn inspect_drivers(&self) -> Result<Vec<Arc<dyn Driver<L, VL>>>> {
        self.custom_drivers
            .iter()
            .map(|registration| registration.descriptor())
            .chain(
                builtin_registrations()
                    .iter()
                    .map(|registration| registration.descriptor),
            )
            .map(|descriptor| self.resolve_driver(descriptor))
            .collect()
    }
}

fn driver_refs<L, VL>(drivers: &[Arc<dyn Driver<L, VL>>]) -> Vec<&dyn Driver<L, VL>>
where
    L: elemental_core::storage::layout::Layout,
    VL: elemental_core::storage::layout::Layout,
{
    drivers.iter().map(Arc::as_ref).collect()
}

fn summarize_installed_instance<L, VL>(instance: InstalledInstance<L, VL>) -> Instance
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let instance_name = instance.storage.name().unwrap_or_else(|| {
        instance
            .storage
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| instance.storage.path.display().to_string())
    });

    Instance {
        instance_name,
        instance_root: instance.storage.path,
        driver: instance.driver,
    }
}

#[cfg(all(test, feature = "driver-vanilla"))]
mod tests {
    use elemental_driver::{descriptors::VANILLA_DRIVER, drivers::vanilla::driver::VanillaDriver};

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
            ))
            .unwrap();

        assert!(matches!(
            launcher.clone().register_driver(DriverRegistration::new(
                custom,
                Arc::new(VanillaDriver::with_defaults()),
            )),
            Err(Error::DriverAlreadyRegistered { .. })
        ));
        assert!(matches!(
            launcher.register_driver(DriverRegistration::new(
                VANILLA_DRIVER,
                Arc::new(VanillaDriver::with_defaults()),
            )),
            Err(Error::DriverAlreadyRegistered { .. })
        ));
    }
}
