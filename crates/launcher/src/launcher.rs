use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use elemental_core::{
    auth::authorizer::Authorizer, launcher::process, minecraft::MinecraftVersionId,
    storage::Storage,
};
use elemental_driver::{
    catalog::{GameVersion, GameVersionCatalog, LoaderRelease, LoaderVersionCatalog},
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
        vanilla::{driver::VanillaDriver, source::VanillaSource},
    },
    families::{
        installer::{InstallerFamily, InstallerFamilyDriver},
        version_json::{
            BaseInstanceLayout, BaseRootLayout, ProfiledVersionJsonDriver,
            ProfiledVersionJsonFamily, ProfiledVersionJsonFamilyExt, VersionJsonGameStorageExt,
            VersionJsonInstanceLayout, VersionJsonRootLayout, build_version_json_launch_command,
            inspect_instances,
        },
    },
    inspect::InstalledInstance,
    loader_version::LoaderVersionId,
};
use elemental_infra::downloader::{DownloadSession, ElementalDownloader};

use crate::{
    builder::LauncherBuilder,
    error::{Error, Result},
    request::{LaunchOptions, PrepareInstanceRequest},
    result::{
        Instance, LaunchCommandResult, LaunchedInstance, PreparedInstance, PreparedInstanceKind,
    },
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

    pub async fn game_versions<C: GameVersionCatalog>(
        &self,
        catalog: C,
    ) -> Result<Vec<GameVersion>> {
        Ok(catalog.game_versions().await?)
    }

    pub async fn loader_versions<C: LoaderVersionCatalog>(
        &self,
        catalog: C,
        game_version: &MinecraftVersionId,
    ) -> Result<Vec<LoaderRelease>> {
        Ok(catalog.loader_versions(game_version).await?)
    }

    pub async fn prepare_instance(
        &self,
        session: &DownloadSession,
        request: PrepareInstanceRequest,
    ) -> Result<PreparedInstance<L, VL>> {
        let instance = self.ensure_instance(request.instance_name).await?;
        let driver_spec = request.driver;
        let prepared_kind = self
            .resolve_driver(driver_spec.descriptor())?
            .prepare(session, &instance, &driver_spec)
            .await?;
        Ok(PreparedInstance::new(
            driver_spec.descriptor(),
            prepared_kind,
        ))
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

    pub async fn build_launch_command<A>(
        &self,
        prepared: &PreparedInstance<L, VL>,
        authorizer: A,
        options: &LaunchOptions,
    ) -> Result<LaunchCommandResult>
    where
        A: Authorizer,
    {
        let (runtime, command) = match &prepared.inner {
            PreparedInstanceKind::VersionJson(version) => {
                build_version_json_launch_command(authorizer, version, options).await?
            }
            PreparedInstanceKind::Installer(version) => {
                build_version_json_launch_command(authorizer, &version.launch_version, options)
                    .await?
            }
        };
        Ok(LaunchCommandResult { runtime, command })
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
        VanillaDriver::new(VanillaSource::default())
    }

    fn profiled_driver<F>(&self, family: F) -> Result<ProfiledVersionJsonDriver<F>>
    where
        F: ProfiledVersionJsonFamily,
        F::Source: Default,
    {
        Ok(family.clone().build_driver(family.default_source()?))
    }

    fn installer_driver<F>(&self) -> InstallerFamilyDriver<F>
    where
        F: InstallerFamily,
        F::Source: Default,
    {
        InstallerFamilyDriver::new(F::Source::default())
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
                ResolvedLauncherDriver::Installer(InstallerDriver::Forge(self.installer_driver()))
            }
            CLEANROOM_DRIVER => ResolvedLauncherDriver::Installer(InstallerDriver::Cleanroom(
                self.installer_driver(),
            )),
            OUTLANDS_DRIVER => ResolvedLauncherDriver::Installer(InstallerDriver::Outlands(
                self.installer_driver(),
            )),
            NEOFORGE_DRIVER => ResolvedLauncherDriver::Installer(InstallerDriver::NeoForge(
                self.installer_driver(),
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
    ) -> Result<PreparedInstanceKind<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match (self, spec.flow()) {
            (Self::Vanilla(driver), DriverFlow::Vanilla) => {
                let prepared = driver
                    .prepare(session, instance, spec.game_version().clone())
                    .await?;
                Ok(PreparedInstanceKind::VersionJson(Box::new(prepared)))
            }
            (Self::Profiled(driver), DriverFlow::Profiled) => {
                let prepared = driver
                    .prepare(session, instance, loader_request(spec)?)
                    .await?;
                Ok(PreparedInstanceKind::VersionJson(Box::new(prepared)))
            }
            (Self::Installer(driver), DriverFlow::Installer) => {
                let prepared = driver
                    .prepare(session, instance, loader_request(spec)?)
                    .await?;
                Ok(PreparedInstanceKind::Installer(Box::new(prepared)))
            }
            _ => Err(Error::UnsupportedDriverFlow {
                id: spec.id().to_owned(),
            }),
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
            Self::Vanilla(driver) => Ok(PreparedInstanceKind::VersionJson(Box::new(
                driver.load_prepared(instance).await?,
            ))),
            Self::Profiled(driver) => Ok(PreparedInstanceKind::VersionJson(Box::new(
                driver.load_prepared(instance).await?,
            ))),
            Self::Installer(driver) => Ok(PreparedInstanceKind::Installer(Box::new(
                driver.load_prepared(instance).await?,
            ))),
        }
    }
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
            Box::new(self.installer_driver::<ForgeFamily>()),
            Box::new(self.installer_driver::<CleanroomDriverSpec>()),
            Box::new(self.installer_driver::<OutlandsDriverSpec>()),
            Box::new(self.installer_driver::<NeoForgeFamily>()),
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
