use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use elemental_core::{
    auth::credential::UserCredential,
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
    driver::{Driver, DriverDescriptor},
    drivers::{
        cleanroom::CleanroomDriverSpec,
        dispatch::{InstallerDriver, LoaderRequest, ProfiledDriver},
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
            BaseInstanceLayout, BaseRootLayout, PreparedVersionJsonInstance,
            ProfiledVersionJsonDriver, ProfiledVersionJsonFamilyExt, VersionJsonGameStorageExt,
            VersionJsonInstanceLayout, VersionJsonRootLayout, builder::VersionJsonLaunchBuilder,
            inspect_instances,
        },
    },
    inspect::InstalledInstance,
    launch::resolve_prepared_version_runtime,
    loader_version::LoaderVersionId,
};
use elemental_infra::downloader::{DownloadSession, ElementalDownloader};

use crate::{
    builder::LauncherBuilder,
    error::{Error, Result},
    origin::{DriverOrigins, InstallerFamilySource, ProfiledFamilySource},
    preparation::InstancePreparation,
    reporter::ProgressReporter,
    request::{LaunchOptions, RuntimeOptions},
    result::{Instance, LaunchedInstance, LoggedLaunchedInstance, PreparedInstance},
    spec::{DriverFlow, DriverSpec},
};

type LauncherGameStorage<L> = Storage<L>;
type LauncherInstanceStorage<L, VL> = Storage<VL, LauncherGameStorage<L>>;

enum ResolvedLauncherDriver {
    Vanilla(VanillaDriver),
    Profiled(ProfiledDriver),
    Installer(InstallerDriver),
}

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

    pub fn new(storage_root: PathBuf, downloader: Arc<ElementalDownloader>) -> Self {
        Self::with_layouts(storage_root, downloader, BaseRootLayout, BaseInstanceLayout)
    }
}

impl<L, VL> Launcher<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone + Send,
{
    pub fn with_layouts(
        storage_root: PathBuf,
        downloader: Arc<ElementalDownloader>,
        root_layout: L,
        instance_layout: VL,
    ) -> Self {
        Self::with_layouts_and_origins(
            storage_root,
            downloader,
            root_layout,
            instance_layout,
            DriverOrigins::default(),
        )
    }

    pub fn with_layouts_and_origins(
        storage_root: PathBuf,
        downloader: Arc<ElementalDownloader>,
        root_layout: L,
        instance_layout: VL,
        origins: DriverOrigins,
    ) -> Self {
        Self::with_layouts_origins_and_runtime(
            storage_root,
            downloader,
            root_layout,
            instance_layout,
            origins,
            RuntimeResolver::with_default_providers(),
        )
    }

    pub fn with_layouts_origins_and_runtime(
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

    pub async fn runtime_distributions(
        &self,
    ) -> Vec<elemental_core::runtime::distribution::Distribution> {
        self.runtime_resolver.distributions().await
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
        self.begin_prepare(instance_name, driver, runtime_options)
            .await?
            .prepare()
            .await
    }

    pub async fn prepare_with_reporter<R>(
        &self,
        instance_name: String,
        driver: DriverSpec,
        runtime_options: &RuntimeOptions,
        reporter: &R,
    ) -> Result<PreparedInstance<L, VL>>
    where
        R: ProgressReporter,
    {
        self.begin_prepare(instance_name, driver, runtime_options)
            .await?
            .prepare_with_reporter(reporter)
            .await
    }

    /// Start a preparation with a default session while exposing its progress.
    ///
    /// Calling [`InstancePreparation::prepare`] keeps session ownership inside
    /// the launcher. Dropping the preparation without calling `prepare` cancels
    /// the download session.
    pub async fn begin_prepare(
        &self,
        instance_name: String,
        driver: DriverSpec,
        runtime_options: &RuntimeOptions,
    ) -> Result<InstancePreparation<'_, L, VL>> {
        let session = self
            .downloader
            .session(format!("prepare {instance_name}"))
            .open()
            .await?;
        Ok(InstancePreparation::new(
            self,
            session,
            instance_name,
            driver,
            runtime_options,
        ))
    }

    /// Install or refresh an instance with a caller-configured session.
    ///
    /// The launcher finishes and validates the session. Use
    /// [`Launcher::prepare_in_session`] when the caller needs to own the
    /// session lifecycle or inspect the download report.
    pub async fn prepare_with_session(
        &self,
        session: DownloadSession,
        instance_name: String,
        driver: DriverSpec,
        runtime_options: &RuntimeOptions,
    ) -> Result<PreparedInstance<L, VL>> {
        InstancePreparation::new(self, session, instance_name, driver, runtime_options)
            .prepare()
            .await
    }

    /// Low-level preparation inside a caller-owned session.
    ///
    /// The caller must finish or cancel `session` and validate its report.
    pub async fn prepare_in_session(
        &self,
        session: &DownloadSession,
        instance_name: String,
        driver: DriverSpec,
        runtime_options: &RuntimeOptions,
    ) -> Result<PreparedInstance<L, VL>> {
        let instance = self.ensure_instance(instance_name).await?;
        let prepared_kind = self
            .resolve_driver(driver.descriptor())?
            .prepare(
                session,
                &instance,
                &driver,
                runtime_options,
                &self.runtime_resolver,
            )
            .await?;
        Ok(PreparedInstance::new(driver.descriptor(), prepared_kind))
    }

    pub async fn load_instance(&self, instance: Instance) -> Result<PreparedInstance<L, VL>> {
        let layout = self.instance(instance.instance_name)?;
        let driver_spec = instance.driver.driver;
        let prepared_kind = self
            .resolve_driver(driver_spec)?
            .load_prepared(&layout)
            .await?;
        Ok(PreparedInstance::new(driver_spec, prepared_kind))
    }

    pub async fn build_launch_command(
        &self,
        prepared: &PreparedInstance<L, VL>,
        credential: &UserCredential,
        runtime_options: &RuntimeOptions,
        options: &LaunchOptions,
    ) -> Result<LaunchCommand> {
        let runtime = resolve_prepared_version_runtime(
            &prepared.launch_version,
            runtime_options,
            &self.runtime_resolver,
            "launch",
        )
        .await?;
        let builder = launch_builder(credential, runtime, prepared, options);
        Ok(builder.build_command()?)
    }

    pub fn launch(&self, command: LaunchCommand) -> Result<LaunchedInstance> {
        let runtime = command.runtime.clone();
        let child = process::spawn_command(command)?;
        Ok(LaunchedInstance { runtime, child })
    }

    pub fn launch_logged(&self, command: LaunchCommand) -> Result<LoggedLaunchedInstance> {
        let runtime = command.runtime.clone();
        let logged = process::spawn_command_logged(command)?;
        Ok(LoggedLaunchedInstance {
            runtime,
            child: logged.child,
            lines: logged.lines,
        })
    }

    pub async fn launch_prepared_instance(
        &self,
        prepared: &PreparedInstance<L, VL>,
        credential: &UserCredential,
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
        credential: &UserCredential,
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

    async fn ensure_instance(
        &self,
        instance_name: String,
    ) -> Result<LauncherInstanceStorage<L, VL>> {
        Ok(self
            .game_storage()
            .ensure_instance(instance_name, self.instance_layout.clone())
            .await?)
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

    fn resolve_driver(&self, desc: DriverDescriptor) -> Result<ResolvedLauncherDriver> {
        Ok(match desc {
            VANILLA_DRIVER => ResolvedLauncherDriver::Vanilla(self.vanilla_driver()),
            FABRIC_DRIVER => ResolvedLauncherDriver::Profiled(ProfiledDriver::FabricLike(
                self.profiled_driver(FabricDriverFamily::new(FabricFlavor::Fabric))?,
            )),
            LEGACY_FABRIC_DRIVER => ResolvedLauncherDriver::Profiled(ProfiledDriver::FabricLike(
                self.profiled_driver(FabricDriverFamily::new(FabricFlavor::LegacyFabric))?,
            )),
            BABRIC_DRIVER => ResolvedLauncherDriver::Profiled(ProfiledDriver::FabricLike(
                self.profiled_driver(FabricDriverFamily::new(FabricFlavor::Babric))?,
            )),
            QUILT_DRIVER => ResolvedLauncherDriver::Profiled(ProfiledDriver::Quilt(
                self.profiled_driver(QuiltDriverFamily)?,
            )),
            LITELOADER_DRIVER => ResolvedLauncherDriver::Profiled(ProfiledDriver::LiteLoader(
                self.profiled_driver(LiteLoaderDriverFamily)?,
            )),
            RIFT_DRIVER => ResolvedLauncherDriver::Profiled(ProfiledDriver::Rift(
                self.profiled_driver(RiftDriverFamily)?,
            )),
            FORGE_DRIVER => {
                ResolvedLauncherDriver::Installer(InstallerDriver::Forge(self.installer_driver()?))
            }
            CLEANROOM_DRIVER => ResolvedLauncherDriver::Installer(InstallerDriver::Cleanroom(
                self.installer_driver()?,
            )),
            OUTLANDS_DRIVER => ResolvedLauncherDriver::Installer(InstallerDriver::Outlands(
                self.installer_driver()?,
            )),
            NEOFORGE_DRIVER => ResolvedLauncherDriver::Installer(InstallerDriver::NeoForge(
                self.installer_driver()?,
            )),
            _ => {
                return Err(Error::UnsupportedDriver {
                    id: desc.id.to_owned(),
                });
            }
        })
    }
}

impl ResolvedLauncherDriver {
    async fn prepare<L, VL>(
        &self,
        session: &DownloadSession,
        instance: &LauncherInstanceStorage<L, VL>,
        spec: &DriverSpec,
        runtime_options: &RuntimeOptions,
        runtime_resolver: &RuntimeResolver,
    ) -> Result<PreparedVersionJsonInstance<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match (self, spec.flow()) {
            (Self::Vanilla(driver), DriverFlow::Vanilla) => {
                let prepared = driver
                    .prepare(session, instance, spec.game_version().clone())
                    .await?;
                Ok(prepared)
            }
            (Self::Profiled(driver), DriverFlow::Profiled) => {
                let prepared = driver
                    .prepare(session, instance, loader_request(spec)?)
                    .await?;
                Ok(prepared)
            }
            (Self::Installer(driver), DriverFlow::Installer) => {
                let prepared = driver
                    .prepare_with_runtime(
                        session,
                        instance,
                        loader_request(spec)?,
                        runtime_options,
                        runtime_resolver,
                    )
                    .await?;
                Ok(prepared.launch_version)
            }
            _ => Err(Error::UnsupportedDriverFlow {
                id: spec.id().to_owned(),
            }),
        }
    }

    async fn load_prepared<L, VL>(
        &self,
        instance: &LauncherInstanceStorage<L, VL>,
    ) -> Result<PreparedVersionJsonInstance<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match self {
            Self::Vanilla(driver) => Ok(driver.load_prepared(instance).await?),
            Self::Profiled(driver) => Ok(driver.load_prepared(instance).await?),
            Self::Installer(driver) => Ok(driver.load_prepared(instance).await?.launch_version),
        }
    }
}

fn launch_builder<L, VL>(
    credential: &UserCredential,
    runtime: Distribution,
    prepared: &PreparedInstance<L, VL>,
    options: &LaunchOptions,
) -> VersionJsonLaunchBuilder<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let mut builder = VersionJsonLaunchBuilder::new(
        credential.clone(),
        runtime,
        prepared.launch_version.version.clone(),
    );

    if let Some(client_id) = &options.client_id {
        builder = builder.set_client_id(client_id.clone());
    }

    if let Some(resolution) = &options.resolution {
        builder = builder.set_resolution(resolution.width.clone(), resolution.height.clone());
    }

    if let (Some(name), Some(version)) = (&options.launcher_name, &options.launcher_version) {
        builder = builder.set_launcher(name.clone(), version.clone());
    }

    if let Some(quick_play) = &options.quick_play {
        builder = builder.set_quick_play(
            quick_play.path.clone(),
            quick_play.multiplayer.clone(),
            quick_play.singleplayer.clone(),
            quick_play.realms.clone(),
        );
    }

    if !options.extra_jvm_arguments.is_empty() {
        builder = builder.set_extra_jvm_arguments(options.extra_jvm_arguments.clone());
    }

    if !options.extra_game_arguments.is_empty() {
        builder = builder.set_extra_game_arguments(options.extra_game_arguments.clone());
    }

    builder
}

fn loader_version(spec: &DriverSpec) -> Result<LoaderVersionId> {
    spec.loader_version()
        .cloned()
        .ok_or_else(|| Error::UnsupportedDriverFlow {
            id: spec.id().to_owned(),
        })
}

fn loader_request(spec: &DriverSpec) -> Result<LoaderRequest> {
    Ok(LoaderRequest {
        game_version: spec.game_version().clone(),
        loader_version: loader_version(spec)?,
    })
}

impl<L, VL> Launcher<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone + Send,
{
    fn inspect_drivers(&self) -> Result<Vec<Box<dyn Driver<L, VL>>>> {
        Ok(vec![
            Box::new(self.profiled_driver(FabricDriverFamily::new(FabricFlavor::Fabric))?),
            Box::new(self.profiled_driver(FabricDriverFamily::new(FabricFlavor::LegacyFabric))?),
            Box::new(self.profiled_driver(FabricDriverFamily::new(FabricFlavor::Babric))?),
            Box::new(self.profiled_driver(QuiltDriverFamily)?),
            Box::new(self.profiled_driver(LiteLoaderDriverFamily)?),
            Box::new(self.profiled_driver(RiftDriverFamily)?),
            Box::new(self.installer_driver::<ForgeFamily>()?),
            Box::new(self.installer_driver::<CleanroomDriverSpec>()?),
            Box::new(self.installer_driver::<OutlandsDriverSpec>()?),
            Box::new(self.installer_driver::<NeoForgeFamily>()?),
            // Vanilla is last so more specific drivers take precedence.
            Box::new(self.vanilla_driver()),
        ])
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
