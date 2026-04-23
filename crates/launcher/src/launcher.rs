use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Result, anyhow, bail};
use elemental_core::{
    auth::authorizer::Authorizer, launcher::process, minecraft::MinecraftVersionId,
    storage::Storage,
};
use elemental_driver::{
    catalog::Catalog,
    driver::Driver,
    drivers::{
        cleanroom::driver::CleanroomDriverSpec,
        fabric::{driver::FabricDriverFamily, source::FabricFlavor},
        forge::prepared::ForgeFamily,
        liteloader::driver::LiteLoaderDriverFamily,
        neoforge::prepared::NeoForgeFamily,
        quilt::driver::QuiltDriverFamily,
        rift::driver::RiftDriverFamily,
        vanilla::{driver::VanillaDriver, source::VanillaSource},
    },
    families::{
        installer::{InstallerFamilyDriver, InstallerFamilyDriverSpec},
        version_json::{
            BaseInstanceLayout, BaseRootLayout, ProfiledVersionJsonDriver,
            ProfiledVersionJsonFamily, ProfiledVersionJsonFamilyExt, VersionJsonGameStorageExt,
            VersionJsonInstanceLayout, VersionJsonRootLayout, inspect_instances,
        },
    },
    inspect::InstalledInstance,
};
use elemental_infra::downloader::core::ElementalDownloader;

use crate::{
    builder::LauncherBuilder,
    request::{LaunchOptions, LoadPreparedInstanceRequest, PrepareInstanceRequest},
    result::{
        Instance, LaunchCommandResult, LaunchedInstance, PreparedInstance, PreparedInstanceKind,
    },
    spec::DriverSpec,
};

type LauncherGameStorage<L> = Storage<L>;
type LauncherInstanceStorage<L, VL> = Storage<VL, LauncherGameStorage<L>>;

struct InspectDrivers {
    vanilla: VanillaDriver,
    fabric: ProfiledVersionJsonDriver<FabricDriverFamily>,
    legacy_fabric: ProfiledVersionJsonDriver<FabricDriverFamily>,
    babric: ProfiledVersionJsonDriver<FabricDriverFamily>,
    quilt: ProfiledVersionJsonDriver<QuiltDriverFamily>,
    liteloader: ProfiledVersionJsonDriver<LiteLoaderDriverFamily>,
    rift: ProfiledVersionJsonDriver<RiftDriverFamily>,
    forge: InstallerFamilyDriver<ForgeFamily>,
    cleanroom: InstallerFamilyDriver<CleanroomDriverSpec>,
    neoforge: InstallerFamilyDriver<NeoForgeFamily>,
}

enum ResolvedLauncherDriver {
    Vanilla(VanillaDriver),
    FabricLike(ProfiledVersionJsonDriver<FabricDriverFamily>),
    Quilt(ProfiledVersionJsonDriver<QuiltDriverFamily>),
    LiteLoader(ProfiledVersionJsonDriver<LiteLoaderDriverFamily>),
    Rift(ProfiledVersionJsonDriver<RiftDriverFamily>),
    Forge(InstallerFamilyDriver<ForgeFamily>),
    Cleanroom(InstallerFamilyDriver<CleanroomDriverSpec>),
    NeoForge(InstallerFamilyDriver<NeoForgeFamily>),
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
        Self {
            storage_root,
            downloader,
            root_layout,
            instance_layout,
        }
    }

    pub fn storage_root(&self) -> &Path {
        &self.storage_root
    }

    pub fn downloader(&self) -> &Arc<ElementalDownloader> {
        &self.downloader
    }

    pub async fn inspect_instances(&self) -> Result<Vec<Instance>> {
        let storage = self.game_storage();
        let inspect_drivers = InspectDrivers::new(self)?;
        let drivers = inspect_drivers.as_array::<L, VL>();
        let mut summaries = inspect_instances(&storage, self.instance_layout.clone(), &drivers)
            .await?
            .into_iter()
            .map(summarize_installed_instance)
            .collect::<Vec<Instance>>();
        summaries.sort_by(|left, right| left.instance_name.cmp(&right.instance_name));
        Ok(summaries)
    }

    pub async fn inspect_instance(&self, instance_name: String) -> Result<Option<Instance>> {
        let instance = self.instance(instance_name)?;
        let inspect_drivers = InspectDrivers::new(self)?;
        let drivers = inspect_drivers.as_array::<L, VL>();

        Ok(InstalledInstance::detect(instance, &drivers)
            .await?
            .map(summarize_installed_instance))
    }

    pub async fn catalog<R, C: Catalog<Release = R>>(
        &self,
        catalog: C,
    ) -> Result<HashMap<MinecraftVersionId, Vec<R>>> {
        catalog.releases().await
    }

    pub async fn prepare_instance(
        &self,
        request: PrepareInstanceRequest,
    ) -> Result<PreparedInstance<L, VL>> {
        let instance = self.ensure_instance(request.instance_name).await?;
        let driver_spec = request.driver;
        let prepared_kind = self
            .resolve_driver(&driver_spec)?
            .prepare(&instance, &driver_spec)
            .await?;
        Ok(PreparedInstance::new(driver_spec, prepared_kind))
    }

    pub async fn load_instance(
        &self,
        request: LoadPreparedInstanceRequest,
    ) -> Result<PreparedInstance<L, VL>> {
        let instance = self.instance(request.instance_name)?;
        let driver_spec = request.driver;
        let prepared_kind = self
            .resolve_driver(&driver_spec)?
            .load_prepared(&instance)
            .await?;
        Ok(PreparedInstance::new(driver_spec, prepared_kind))
    }

    pub async fn build_launch_command<A>(
        &self,
        prepared: &PreparedInstance<L, VL>,
        authorizer: A,
        options: &LaunchOptions,
    ) -> Result<LaunchCommandResult>
    where
        A: Authorizer,
    {
        self.resolve_driver(prepared.driver())?
            .build_launch_command(prepared, authorizer, options)
            .await
    }

    pub async fn launch_prepared_instance<A>(
        &self,
        prepared: &PreparedInstance<L, VL>,
        authorizer: A,
        options: &LaunchOptions,
    ) -> Result<LaunchedInstance>
    where
        A: Authorizer,
    {
        let command = self
            .build_launch_command(prepared, authorizer, options)
            .await?;
        let child = process::spawn_command(command.command.clone())?;

        Ok(LaunchedInstance {
            runtime: command.runtime,
            child,
        })
    }

    fn game_storage(&self) -> LauncherGameStorage<L> {
        Storage::new(self.storage_root.clone(), self.root_layout.clone())
    }

    async fn ensure_instance(
        &self,
        instance_name: String,
    ) -> Result<LauncherInstanceStorage<L, VL>> {
        self.game_storage()
            .ensure_instance(instance_name, self.instance_layout.clone())
            .await
    }

    fn instance(&self, instance_name: String) -> Result<LauncherInstanceStorage<L, VL>> {
        self.game_storage()
            .instance(instance_name, self.instance_layout.clone())
    }

    fn vanilla_driver(&self) -> VanillaDriver {
        VanillaDriver::new(VanillaSource::default(), self.downloader.clone())
    }

    fn profiled_driver<F>(&self, family: F) -> Result<ProfiledVersionJsonDriver<F>>
    where
        F: ProfiledVersionJsonFamily,
    {
        Ok(family.clone().build_driver(
            family.default_source()?,
            VanillaSource::default(),
            self.downloader.clone(),
        ))
    }

    fn installer_driver<F>(&self) -> InstallerFamilyDriver<F>
    where
        F: InstallerFamilyDriverSpec,
        F::Source: Default,
    {
        InstallerFamilyDriver::new(
            F::Source::default(),
            VanillaSource::default(),
            self.downloader.clone(),
        )
    }

    fn resolve_driver(&self, spec: &DriverSpec) -> Result<ResolvedLauncherDriver> {
        Ok(match spec {
            DriverSpec::Vanilla(_) => ResolvedLauncherDriver::Vanilla(self.vanilla_driver()),
            DriverSpec::Fabric(_) => ResolvedLauncherDriver::FabricLike(
                self.profiled_driver(FabricDriverFamily::new(FabricFlavor::Fabric))?,
            ),
            DriverSpec::LegacyFabric(_) => ResolvedLauncherDriver::FabricLike(
                self.profiled_driver(FabricDriverFamily::new(FabricFlavor::LegacyFabric))?,
            ),
            DriverSpec::Babric(_) => ResolvedLauncherDriver::FabricLike(
                self.profiled_driver(FabricDriverFamily::new(FabricFlavor::Babric))?,
            ),
            DriverSpec::Quilt(_) => {
                ResolvedLauncherDriver::Quilt(self.profiled_driver(QuiltDriverFamily)?)
            }
            DriverSpec::LiteLoader(_) => {
                ResolvedLauncherDriver::LiteLoader(self.profiled_driver(LiteLoaderDriverFamily)?)
            }
            DriverSpec::Rift(_) => {
                ResolvedLauncherDriver::Rift(self.profiled_driver(RiftDriverFamily)?)
            }
            DriverSpec::Forge(_) => ResolvedLauncherDriver::Forge(self.installer_driver()),
            DriverSpec::Cleanroom(_) => ResolvedLauncherDriver::Cleanroom(self.installer_driver()),
            DriverSpec::NeoForge(_) => ResolvedLauncherDriver::NeoForge(self.installer_driver()),
        })
    }
}

impl ResolvedLauncherDriver {
    async fn prepare<L, VL>(
        &self,
        instance: &LauncherInstanceStorage<L, VL>,
        spec: &DriverSpec,
    ) -> Result<PreparedInstanceKind<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match (self, spec) {
            (Self::Vanilla(driver), DriverSpec::Vanilla(vanilla_spec)) => {
                let prepared = driver
                    .prepare(instance, vanilla_spec.game_version.clone())
                    .await?;
                Ok(PreparedInstanceKind::Vanilla(prepared))
            }
            (Self::FabricLike(driver), DriverSpec::Fabric(loader_spec))
            | (Self::FabricLike(driver), DriverSpec::LegacyFabric(loader_spec))
            | (Self::FabricLike(driver), DriverSpec::Babric(loader_spec)) => {
                let prepared = driver
                    .prepare(
                        instance,
                        loader_spec.game_version.clone(),
                        loader_spec.loader_version.clone(),
                    )
                    .await?;
                Ok(PreparedInstanceKind::FabricLike(prepared))
            }
            (Self::Quilt(driver), DriverSpec::Quilt(loader_spec)) => {
                let prepared = driver
                    .prepare(
                        instance,
                        loader_spec.game_version.clone(),
                        loader_spec.loader_version.clone(),
                    )
                    .await?;
                Ok(PreparedInstanceKind::Quilt(prepared))
            }
            (Self::LiteLoader(driver), DriverSpec::LiteLoader(loader_spec)) => {
                let prepared = driver
                    .prepare(
                        instance,
                        loader_spec.game_version.clone(),
                        loader_spec.loader_version.clone(),
                    )
                    .await?;
                Ok(PreparedInstanceKind::LiteLoader(prepared))
            }
            (Self::Rift(driver), DriverSpec::Rift(loader_spec)) => {
                let prepared = driver
                    .prepare(
                        instance,
                        loader_spec.game_version.clone(),
                        loader_spec.loader_version.clone(),
                    )
                    .await?;
                Ok(PreparedInstanceKind::Rift(prepared))
            }
            (Self::Forge(driver), DriverSpec::Forge(loader_spec)) => {
                let prepared = driver
                    .prepare(
                        instance,
                        loader_spec.game_version.clone(),
                        loader_spec.loader_version.clone(),
                    )
                    .await?;
                Ok(PreparedInstanceKind::Forge(prepared))
            }
            (Self::Cleanroom(driver), DriverSpec::Cleanroom(loader_spec)) => {
                let prepared = driver
                    .prepare(
                        instance,
                        loader_spec.game_version.clone(),
                        loader_spec.loader_version.clone(),
                    )
                    .await?;
                Ok(PreparedInstanceKind::Cleanroom(prepared))
            }
            (Self::NeoForge(driver), DriverSpec::NeoForge(loader_spec)) => {
                let prepared = driver
                    .prepare(
                        instance,
                        loader_spec.game_version.clone(),
                        loader_spec.loader_version.clone(),
                    )
                    .await?;
                Ok(PreparedInstanceKind::NeoForge(prepared))
            }
            _ => bail!(
                "resolved launcher driver does not support '{}' prepare flow",
                spec.id()
            ),
        }
    }

    async fn load_prepared<L, VL>(
        &self,
        instance: &LauncherInstanceStorage<L, VL>,
    ) -> Result<PreparedInstanceKind<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match self {
            Self::Vanilla(driver) => Ok(PreparedInstanceKind::Vanilla(
                driver.load_prepared(instance).await?,
            )),
            Self::FabricLike(driver) => Ok(PreparedInstanceKind::FabricLike(
                driver.load_prepared(instance).await?,
            )),
            Self::Quilt(driver) => Ok(PreparedInstanceKind::Quilt(
                driver.load_prepared(instance).await?,
            )),
            Self::LiteLoader(driver) => Ok(PreparedInstanceKind::LiteLoader(
                driver.load_prepared(instance).await?,
            )),
            Self::Rift(driver) => Ok(PreparedInstanceKind::Rift(
                driver.load_prepared(instance).await?,
            )),
            Self::Forge(driver) => Ok(PreparedInstanceKind::Forge(
                driver.load_prepared(instance).await?,
            )),
            Self::Cleanroom(driver) => Ok(PreparedInstanceKind::Cleanroom(
                driver.load_prepared(instance).await?,
            )),
            Self::NeoForge(driver) => Ok(PreparedInstanceKind::NeoForge(
                driver.load_prepared(instance).await?,
            )),
        }
    }

    async fn build_launch_command<A, L, VL>(
        &self,
        prepared: &PreparedInstance<L, VL>,
        authorizer: A,
        launch_options: &LaunchOptions,
    ) -> Result<LaunchCommandResult>
    where
        A: Authorizer,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match self {
            Self::Vanilla(driver) => {
                let PreparedInstanceKind::Vanilla(version) = &prepared.inner else {
                    return Err(prepared_variant_mismatch(prepared));
                };
                let (runtime, command) = driver
                    .build_launch_command(authorizer, version, launch_options)
                    .await?;
                Ok(LaunchCommandResult { runtime, command })
            }
            Self::FabricLike(driver) => {
                let PreparedInstanceKind::FabricLike(version) = &prepared.inner else {
                    return Err(prepared_variant_mismatch(prepared));
                };
                let (runtime, command) = driver
                    .build_launch_command(authorizer, version, launch_options)
                    .await?;
                Ok(LaunchCommandResult { runtime, command })
            }
            Self::Quilt(driver) => {
                let PreparedInstanceKind::Quilt(version) = &prepared.inner else {
                    return Err(prepared_variant_mismatch(prepared));
                };
                let (runtime, command) = driver
                    .build_launch_command(authorizer, version, launch_options)
                    .await?;
                Ok(LaunchCommandResult { runtime, command })
            }
            Self::LiteLoader(driver) => {
                let PreparedInstanceKind::LiteLoader(version) = &prepared.inner else {
                    return Err(prepared_variant_mismatch(prepared));
                };
                let (runtime, command) = driver
                    .build_launch_command(authorizer, version, launch_options)
                    .await?;
                Ok(LaunchCommandResult { runtime, command })
            }
            Self::Rift(driver) => {
                let PreparedInstanceKind::Rift(version) = &prepared.inner else {
                    return Err(prepared_variant_mismatch(prepared));
                };
                let (runtime, command) = driver
                    .build_launch_command(authorizer, version, launch_options)
                    .await?;
                Ok(LaunchCommandResult { runtime, command })
            }
            Self::Forge(driver) => {
                let PreparedInstanceKind::Forge(version) = &prepared.inner else {
                    return Err(prepared_variant_mismatch(prepared));
                };
                let (runtime, command) = driver
                    .build_launch_command(authorizer, version, launch_options)
                    .await?;
                Ok(LaunchCommandResult { runtime, command })
            }
            Self::Cleanroom(driver) => {
                let PreparedInstanceKind::Cleanroom(version) = &prepared.inner else {
                    return Err(prepared_variant_mismatch(prepared));
                };
                let (runtime, command) = driver
                    .build_launch_command(authorizer, version, launch_options)
                    .await?;
                Ok(LaunchCommandResult { runtime, command })
            }
            Self::NeoForge(driver) => {
                let PreparedInstanceKind::NeoForge(version) = &prepared.inner else {
                    return Err(prepared_variant_mismatch(prepared));
                };
                let (runtime, command) = driver
                    .build_launch_command(authorizer, version, launch_options)
                    .await?;
                Ok(LaunchCommandResult { runtime, command })
            }
        }
    }
}

fn prepared_variant_mismatch<L, VL>(prepared: &PreparedInstance<L, VL>) -> anyhow::Error
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    anyhow!(
        "prepared instance variant does not match driver '{}'",
        prepared.driver.id()
    )
}

impl InspectDrivers {
    fn new<L, VL>(launcher: &Launcher<L, VL>) -> Result<Self>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone + Send,
    {
        Ok(Self {
            vanilla: launcher.vanilla_driver(),
            fabric: launcher.profiled_driver(FabricDriverFamily::new(FabricFlavor::Fabric))?,
            legacy_fabric: launcher
                .profiled_driver(FabricDriverFamily::new(FabricFlavor::LegacyFabric))?,
            babric: launcher.profiled_driver(FabricDriverFamily::new(FabricFlavor::Babric))?,
            quilt: launcher.profiled_driver(QuiltDriverFamily)?,
            liteloader: launcher.profiled_driver(LiteLoaderDriverFamily)?,
            rift: launcher.profiled_driver(RiftDriverFamily)?,
            forge: launcher.installer_driver::<ForgeFamily>(),
            cleanroom: launcher.installer_driver::<CleanroomDriverSpec>(),
            neoforge: launcher.installer_driver::<NeoForgeFamily>(),
        })
    }

    fn as_array<L, VL>(&self) -> [&dyn Driver<L, VL>; 10]
    where
        L: elemental_core::storage::layout::Layout,
        VL: elemental_core::storage::layout::Layout,
    {
        [
            &self.fabric,
            &self.legacy_fabric,
            &self.babric,
            &self.quilt,
            &self.liteloader,
            &self.rift,
            &self.forge,
            &self.cleanroom,
            &self.neoforge,
            // Every instance could be a vanilla instance, so the vanilla driver should be last to allow other drivers to take precedence when applicable
            &self.vanilla,
        ]
    }
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
