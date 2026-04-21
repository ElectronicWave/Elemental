use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use elemental_core::{
    auth::authorizer::Authorizer,
    launcher::{command::LaunchCommand, process},
    runtime::distribution::Distribution,
    storage::{Storage, layout::Layout},
};
use elemental_infra::downloader::core::ElementalDownloader;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::{
        forge::{
            config::ForgeLaunchConfig,
            prepared::{
                ForgeRemoteResolver, PreparedForgeLaunchVersion, PreparedForgeVersion,
                ResolvedForgeVersion,
            },
            source::ForgeSource,
        },
        vanilla::source::VanillaSource,
    },
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonRootLayout, builder::VersionJsonLaunchBuilder,
    },
    inspect::InstanceProbe,
    launch::{build_version_json_launch_builder, resolve_prepared_version_runtime},
};

const FORGE_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "forge",
    name: "Forge",
};

pub struct ForgeDriver {
    source: ForgeSource,
    vanilla_source: VanillaSource,
    downloader: Arc<ElementalDownloader>,
}

pub struct LaunchedForgeVersion<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub prepared_version: PreparedForgeVersion<L, VL>,
    pub runtime: Distribution,
    pub child: tokio::process::Child,
}

impl ForgeDriver {
    pub fn new(
        source: ForgeSource,
        vanilla_source: VanillaSource,
        downloader: Arc<ElementalDownloader>,
    ) -> Self {
        Self {
            source,
            vanilla_source,
            downloader,
        }
    }

    pub fn with_defaults() -> Result<Self> {
        Ok(Self::new(
            ForgeSource::default(),
            VanillaSource::default(),
            ElementalDownloader::with_config_default()
                .context("create default elemental downloader failed")?,
        ))
    }

    pub fn source(&self) -> &ForgeSource {
        &self.source
    }

    pub fn vanilla_source(&self) -> &VanillaSource {
        &self.vanilla_source
    }

    pub fn downloader(&self) -> &ElementalDownloader {
        self.downloader.as_ref()
    }

    pub async fn prepare<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
        game_version: String,
        loader_version: String,
    ) -> Result<PreparedForgeVersion<L, VL>> {
        self.prepare_with_config(
            instance,
            game_version,
            loader_version,
            &ForgeLaunchConfig::new(),
        )
        .await
    }

    pub async fn prepare_with_config<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
        game_version: String,
        loader_version: String,
        config: &ForgeLaunchConfig,
    ) -> Result<PreparedForgeVersion<L, VL>> {
        let installer_artifact =
            self.source
                .installer_artifact(&instance.parent, &game_version, &loader_version)?;

        ResolvedForgeVersion {
            source: self.source.clone(),
            instance: instance.clone(),
            game_version,
            loader_version,
            installer_artifact,
        }
        .prepare(
            self.downloader(),
            self.vanilla_source(),
            &self.remote_resolver(),
            config.runtime_executable_path.as_deref(),
        )
        .await
    }

    pub async fn load_prepared<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedForgeVersion<L, VL>> {
        ResolvedForgeVersion::load(
            self.source.clone(),
            self.remote_resolver(),
            instance.clone(),
        )
        .await
    }

    pub async fn launch<
        A,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        prepared_version: PreparedForgeVersion<L, VL>,
        config: &ForgeLaunchConfig,
        authorizer: A,
    ) -> Result<LaunchedForgeVersion<L, VL>>
    where
        A: Authorizer,
    {
        let (runtime, command) = self
            .build_launch_command(authorizer, &prepared_version, config)
            .await?;
        let child = process::spawn_command(command)?;

        Ok(LaunchedForgeVersion {
            prepared_version,
            runtime,
            child,
        })
    }

    pub async fn build_launch_command<
        A,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        authorizer: A,
        prepared_version: &PreparedForgeVersion<L, VL>,
        config: &ForgeLaunchConfig,
    ) -> Result<(Distribution, LaunchCommand)>
    where
        A: Authorizer,
    {
        let runtime = resolve_prepared_version_runtime(
            &prepared_version.launch_version,
            config.runtime_major_version,
            config.runtime_executable_path.as_deref(),
        )
        .await?;
        let command = self
            .build_launch_builder(
                authorizer,
                runtime.clone(),
                &prepared_version.launch_version,
                config,
            )?
            .build_command()
            .await?;

        Ok((runtime, command))
    }

    fn build_launch_builder<
        A,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        authorizer: A,
        runtime: Distribution,
        prepared_version: &PreparedForgeLaunchVersion<L, VL>,
        config: &ForgeLaunchConfig,
    ) -> Result<VersionJsonLaunchBuilder<A, L, VL>>
    where
        A: Authorizer,
    {
        build_version_json_launch_builder(authorizer, runtime, prepared_version, config)
    }

    fn remote_resolver(&self) -> ForgeRemoteResolver {
        ForgeRemoteResolver::new(
            self.vanilla_source.endpoints().clone(),
            self.source.endpoints().clone(),
        )
    }
}

#[async_trait]
impl<L: Layout, VL: Layout> Driver<L, VL> for ForgeDriver {
    fn descriptor(&self) -> DriverDescriptor {
        FORGE_DRIVER
    }

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };
        let library_name = metadata
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .find(|name| name.starts_with("net.minecraftforge:fmlloader:"));

        let Some(library_name) = library_name else {
            return Ok(None);
        };

        let driver_version = library_name.split(':').nth(2).map(ToOwned::to_owned);

        Ok(Some(InstalledDriver {
            driver: FORGE_DRIVER,
            driver_version,
            game_version: metadata
                .inherits_from
                .clone()
                .or_else(|| Some(metadata.id.clone())),
            description: Some(metadata.release_type.clone()),
        }))
    }
}
