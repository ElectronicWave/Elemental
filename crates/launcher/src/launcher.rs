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
    descriptors::{
        BABRIC_DRIVER, CLEANROOM_DRIVER, FABRIC_DRIVER, FORGE_DRIVER, LEGACY_FABRIC_DRIVER,
        LITELOADER_DRIVER, NEOFORGE_DRIVER, OUTLANDS_DRIVER, QUILT_DRIVER, RIFT_DRIVER,
        VANILLA_DRIVER,
    },
    driver::{Driver, DriverDescriptor, PrepareRequest},
    drivers::{
        cleanroom::CleanroomDriverSpec,
        fabric::{driver::FabricDriverFamily, source::FabricFlavor},
        forge::ForgeFamily,
        liteloader::driver::LiteLoaderDriverFamily,
        neoforge::NeoForgeFamily,
        outlands::OutlandsDriverSpec,
        quilt::driver::QuiltDriverFamily,
        rift::driver::RiftDriverFamily,
        vanilla::driver::VanillaDriver,
    },
    families::{
        installer::InstallerFamilyDriver,
        version_json::{
            BaseInstanceLayout, BaseRootLayout, ProfiledVersionJsonDriver,
            ProfiledVersionJsonFamilyExt, VersionJsonGameStorageExt, VersionJsonInstanceLayout,
            VersionJsonRootLayout, builder::VersionJsonLaunchBuilder, inspect_instances,
            variables::LauncherVariables,
        },
    },
    inspect::InstalledInstance,
    launch::resolve_prepared_version_runtime,
};
use elemental_infra::downloader::{DownloadSession, ElementalDownloader};

use crate::{
    builder::LauncherBuilder,
    error::{Error, Result},
    origin::{DriverOrigins, InstallerFamilySource, ProfiledFamilySource},
    request::{LaunchOptions, RuntimeOptions},
    result::{Instance, LaunchedInstance, PreparedInstance},
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
        }
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

    pub async fn inspect_instances(&self) -> Result<Vec<Instance>> {
        let storage = self.game_storage();
        let drivers = self.inspect_drivers()?;
        let driver_refs = driver_refs(&drivers);
        let mut summaries = inspect_instances(&storage, self.instance_layout.clone(), &driver_refs)
            .await?
            .into_iter()
            .map(summarize_installed_instance)
            .collect::<Vec<Instance>>();
        summaries.sort_by(|left, right| left.instance_name.cmp(&right.instance_name));
        Ok(summaries)
    }

    pub async fn inspect_instance(&self, instance_name: String) -> Result<Option<Instance>> {
        let instance = self.instance(instance_name)?;
        let drivers = self.inspect_drivers()?;
        let driver_refs = driver_refs(&drivers);

        Ok(InstalledInstance::detect(instance, &driver_refs)
            .await?
            .map(summarize_installed_instance))
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

    fn vanilla_driver(&self) -> VanillaDriver {
        VanillaDriver::new(self.origins.vanilla_source())
    }

    fn profiled_driver<F>(&self, family: F) -> Result<ProfiledVersionJsonDriver<F>>
    where
        F: ProfiledFamilySource,
    {
        Ok(family
            .clone()
            .build_driver(family.source_with_origins(&self.origins)?))
    }

    fn installer_driver<F>(&self) -> Result<InstallerFamilyDriver<F>>
    where
        F: InstallerFamilySource,
    {
        Ok(InstallerFamilyDriver::new(F::source_with_origins(
            &self.origins,
        )?))
    }

    fn resolve_driver(&self, desc: DriverDescriptor) -> Result<Box<dyn Driver<L, VL>>> {
        if desc == VANILLA_DRIVER {
            return Ok(Box::new(self.vanilla_driver()));
        }
        if let Some(driver) = self.resolve_profiled_driver(desc)? {
            return Ok(driver);
        }
        if let Some(driver) = self.resolve_installer_driver(desc)? {
            return Ok(driver);
        }
        Err(Error::UnsupportedDriver {
            id: desc.id.to_owned(),
        })
    }

    fn resolve_profiled_driver(
        &self,
        desc: DriverDescriptor,
    ) -> Result<Option<Box<dyn Driver<L, VL>>>> {
        Ok(Some(match desc {
            FABRIC_DRIVER => {
                Box::new(self.profiled_driver(FabricDriverFamily::new(FabricFlavor::Fabric))?)
            }
            LEGACY_FABRIC_DRIVER => {
                Box::new(self.profiled_driver(FabricDriverFamily::new(FabricFlavor::LegacyFabric))?)
            }
            BABRIC_DRIVER => {
                Box::new(self.profiled_driver(FabricDriverFamily::new(FabricFlavor::Babric))?)
            }
            QUILT_DRIVER => Box::new(self.profiled_driver(QuiltDriverFamily)?),
            LITELOADER_DRIVER => Box::new(self.profiled_driver(LiteLoaderDriverFamily)?),
            RIFT_DRIVER => Box::new(self.profiled_driver(RiftDriverFamily)?),
            _ => return Ok(None),
        }))
    }

    fn resolve_installer_driver(
        &self,
        desc: DriverDescriptor,
    ) -> Result<Option<Box<dyn Driver<L, VL>>>> {
        Ok(Some(match desc {
            FORGE_DRIVER => Box::new(self.installer_driver::<ForgeFamily>()?),
            CLEANROOM_DRIVER => Box::new(self.installer_driver::<CleanroomDriverSpec>()?),
            OUTLANDS_DRIVER => Box::new(self.installer_driver::<OutlandsDriverSpec>()?),
            NEOFORGE_DRIVER => Box::new(self.installer_driver::<NeoForgeFamily>()?),
            _ => return Ok(None),
        }))
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
    fn inspect_drivers(&self) -> Result<Vec<Box<dyn Driver<L, VL>>>> {
        [
            FABRIC_DRIVER,
            LEGACY_FABRIC_DRIVER,
            BABRIC_DRIVER,
            QUILT_DRIVER,
            LITELOADER_DRIVER,
            RIFT_DRIVER,
            FORGE_DRIVER,
            CLEANROOM_DRIVER,
            OUTLANDS_DRIVER,
            NEOFORGE_DRIVER,
            // Vanilla is last so more specific drivers take precedence.
            VANILLA_DRIVER,
        ]
        .into_iter()
        .map(|descriptor| self.resolve_driver(descriptor))
        .collect()
    }
}

fn driver_refs<L, VL>(drivers: &[Box<dyn Driver<L, VL>>]) -> Vec<&dyn Driver<L, VL>>
where
    L: elemental_core::storage::layout::Layout,
    VL: elemental_core::storage::layout::Layout,
{
    drivers.iter().map(Box::as_ref).collect()
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
