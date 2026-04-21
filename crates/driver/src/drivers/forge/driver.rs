use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use elemental_core::{
    auth::authorizer::Authorizer,
    launcher::command::LaunchCommand,
    runtime::distribution::Distribution,
    storage::{Storage, layout::Layout},
};
use elemental_infra::downloader::core::ElementalDownloader;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::{
        forge::{
            config::ForgeLaunchConfig,
            prepared::{ForgeRemoteResolver, PreparedForgeVersion, ResolvedForgeVersion},
            source::ForgeSource,
        },
        shared::{
            build_version_json_launch_command, find_library_version, installed_version_json_driver,
            launch_wrapped_version,
        },
        vanilla::source::VanillaSource,
    },
    families::version_json::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    inspect::InstanceProbe,
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
        launch_wrapped_version(
            authorizer,
            prepared_version,
            config,
            |prepared_version| &prepared_version.launch_version,
            |prepared_version, runtime, child| LaunchedForgeVersion {
                prepared_version,
                runtime,
                child,
            },
        )
        .await
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
        build_version_json_launch_command(authorizer, &prepared_version.launch_version, config)
            .await
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
        let Some(driver_version) =
            find_library_version(metadata, &["net.minecraftforge:fmlloader:"])
        else {
            return Ok(None);
        };

        Ok(Some(installed_version_json_driver(
            metadata,
            FORGE_DRIVER,
            Some(driver_version),
        )))
    }
}
