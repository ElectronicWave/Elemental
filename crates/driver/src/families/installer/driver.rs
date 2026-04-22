use std::{marker::PhantomData, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;
use elemental_core::{
    auth::authorizer::Authorizer,
    launcher::command::LaunchCommand,
    minecraft::MinecraftVersionId,
    runtime::distribution::Distribution,
    storage::{Storage, layout::Layout},
};
use elemental_infra::downloader::core::ElementalDownloader;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::vanilla::source::VanillaSource,
    families::{
        installer::{
            InstallerFamily, InstallerFamilyRemoteResolver, PreparedInstallerFamilyVersion,
            ResolvedInstallerFamilyVersion,
        },
        version_json::{
            VersionJsonInstanceLayout, VersionJsonLaunchConfig, VersionJsonRootLayout,
            build_version_json_launch_command, launch_wrapped_version,
        },
    },
    inspect::{InstanceProbe, inspect_driver_version_from_libraries},
    loader_version::LoaderVersionId,
};

pub trait InstallerFamilyDriverSpec: InstallerFamily {
    const DRIVER: DriverDescriptor;
    const INSPECT_PREFIXES: &'static [&'static str];
}

#[derive(Debug, Clone)]
pub struct InstallerFamilyDriver<F>
where
    F: InstallerFamilyDriverSpec,
{
    source: F::Source,
    vanilla_source: VanillaSource,
    downloader: Arc<ElementalDownloader>,
    family: PhantomData<F>,
}

pub struct LaunchedInstallerFamilyVersion<F, L, VL>
where
    F: InstallerFamilyDriverSpec,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub prepared_version: PreparedInstallerFamilyVersion<F, L, VL>,
    pub runtime: Distribution,
    pub child: tokio::process::Child,
}

impl<F> InstallerFamilyDriver<F>
where
    F: InstallerFamilyDriverSpec,
{
    pub fn new(
        source: F::Source,
        vanilla_source: VanillaSource,
        downloader: Arc<ElementalDownloader>,
    ) -> Self {
        Self {
            source,
            vanilla_source,
            downloader,
            family: PhantomData,
        }
    }

    pub fn source(&self) -> &F::Source {
        &self.source
    }

    pub fn vanilla_source(&self) -> &VanillaSource {
        &self.vanilla_source
    }

    pub fn downloader(&self) -> &ElementalDownloader {
        self.downloader.as_ref()
    }

    pub async fn prepare<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<PreparedInstallerFamilyVersion<F, L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        self.prepare_with_config(
            instance,
            game_version,
            loader_version,
            &VersionJsonLaunchConfig::new(),
        )
        .await
    }

    pub async fn prepare_with_config<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
        config: &VersionJsonLaunchConfig,
    ) -> Result<PreparedInstallerFamilyVersion<F, L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        ResolvedInstallerFamilyVersion::new(
            self.source.clone(),
            instance.clone(),
            game_version,
            loader_version,
        )?
        .prepare(
            self.downloader(),
            self.vanilla_source(),
            &self.remote_resolver(),
            config.runtime_executable_path.as_deref(),
            config.runtime_validation,
        )
        .await
    }

    pub async fn load_prepared<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedInstallerFamilyVersion<F, L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        ResolvedInstallerFamilyVersion::load(
            self.source.clone(),
            self.remote_resolver(),
            instance.clone(),
        )
        .await
    }

    pub async fn launch<A, L, VL>(
        &self,
        prepared_version: PreparedInstallerFamilyVersion<F, L, VL>,
        config: &VersionJsonLaunchConfig,
        authorizer: A,
    ) -> Result<LaunchedInstallerFamilyVersion<F, L, VL>>
    where
        A: Authorizer,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        launch_wrapped_version(
            authorizer,
            prepared_version,
            config,
            |prepared_version| &prepared_version.launch_version,
            |prepared_version, runtime, child| LaunchedInstallerFamilyVersion {
                prepared_version,
                runtime,
                child,
            },
        )
        .await
    }

    pub async fn build_launch_command<A, L, VL>(
        &self,
        authorizer: A,
        prepared_version: &PreparedInstallerFamilyVersion<F, L, VL>,
        config: &VersionJsonLaunchConfig,
    ) -> Result<(Distribution, LaunchCommand)>
    where
        A: Authorizer,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        build_version_json_launch_command(authorizer, &prepared_version.launch_version, config)
            .await
    }

    fn remote_resolver(&self) -> InstallerFamilyRemoteResolver<F> {
        InstallerFamilyRemoteResolver::new(
            self.vanilla_source.endpoints().clone(),
            F::source_endpoints(&self.source).clone(),
        )
    }
}

impl<F> InstallerFamilyDriver<F>
where
    F: InstallerFamilyDriverSpec,
    F::Source: Default,
{
    pub fn with_defaults() -> Result<Self> {
        Ok(Self::new(
            F::Source::default(),
            VanillaSource::default(),
            ElementalDownloader::with_config_default()
                .context("create default elemental downloader failed")?,
        ))
    }
}

#[async_trait]
impl<F, L, VL> Driver<L, VL> for InstallerFamilyDriver<F>
where
    F: InstallerFamilyDriverSpec,
    F::Source: Sync,
    L: Layout,
    VL: Layout,
{
    fn descriptor(&self) -> DriverDescriptor {
        F::DRIVER
    }

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };

        Ok(inspect_driver_version_from_libraries(
            metadata,
            F::DRIVER,
            F::INSPECT_PREFIXES,
        ))
    }
}
