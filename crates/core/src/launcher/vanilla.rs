use std::sync::Arc;

use anyhow::{Context, Result};
use elemental_infra::downloader::core::ElementalDownloader;

use crate::{
    auth::authorizer::Authorizer,
    install::{ReadyVanillaVersion, ResolvedVanillaVersion},
    runtime::distribution::Distribution,
    services::mojang::MojangClient,
    storage::{game::GameStorage, layout::Layout},
};

use super::builder::LaunchBuilder;

#[derive(Clone)]
pub struct VanillaVersionSpec<VL: Layout + Clone> {
    pub version_id: String,
    pub version_name: String,
    pub version_layout: VL,
}

#[derive(Clone)]
pub struct LaunchResolution {
    pub width: String,
    pub height: String,
}

#[derive(Clone)]
pub struct QuickPlayOptions {
    pub path: Option<String>,
    pub multiplayer: Option<String>,
    pub singleplayer: Option<String>,
    pub realms: Option<String>,
}

#[derive(Clone)]
pub struct VanillaLaunchOptions<VL: Layout + Clone> {
    pub version: VanillaVersionSpec<VL>,
    pub runtime_major_version: Option<usize>,
    pub launcher_name: Option<String>,
    pub launcher_version: Option<String>,
    pub client_id: Option<String>,
    pub resolution: Option<LaunchResolution>,
    pub quick_play: Option<QuickPlayOptions>,
}

pub struct VanillaLauncher {
    client: MojangClient,
    downloader: Arc<ElementalDownloader>,
}

pub struct LaunchedVanillaVersion<L: Layout, VL: Layout> {
    pub ready_version: ReadyVanillaVersion<L, VL>,
    pub runtime: Distribution,
    pub child: tokio::process::Child,
}

impl<VL: Layout + Clone> VanillaVersionSpec<VL> {
    pub fn new(version_id: String, version_name: String, version_layout: VL) -> Self {
        Self {
            version_id,
            version_name,
            version_layout,
        }
    }
}

impl LaunchResolution {
    pub fn new(width: String, height: String) -> Self {
        Self { width, height }
    }
}

impl QuickPlayOptions {
    pub fn new(
        path: Option<String>,
        multiplayer: Option<String>,
        singleplayer: Option<String>,
        realms: Option<String>,
    ) -> Self {
        Self {
            path,
            multiplayer,
            singleplayer,
            realms,
        }
    }
}

impl<VL: Layout + Clone> VanillaLaunchOptions<VL> {
    pub fn new(version: VanillaVersionSpec<VL>) -> Self {
        Self {
            version,
            runtime_major_version: None,
            launcher_name: None,
            launcher_version: None,
            client_id: None,
            resolution: None,
            quick_play: None,
        }
    }
}

impl VanillaLauncher {
    pub fn new(client: MojangClient, downloader: Arc<ElementalDownloader>) -> Self {
        Self { client, downloader }
    }

    pub fn with_defaults() -> Result<Self> {
        Ok(Self::new(
            MojangClient::default(),
            ElementalDownloader::with_config_default()
                .context("create default elemental downloader failed")?,
        ))
    }

    pub fn client(&self) -> &MojangClient {
        &self.client
    }

    pub fn downloader(&self) -> &ElementalDownloader {
        self.downloader.as_ref()
    }

    pub async fn ready<L: Layout + Clone, VL: Layout + Clone>(
        &self,
        storage: &GameStorage<L>,
        version: &VanillaVersionSpec<VL>,
    ) -> Result<ReadyVanillaVersion<L, VL>> {
        let resolved = self.resolve_or_load(storage, version).await?;
        resolved.ready(self.downloader()).await
    }

    pub async fn launch<A, L: Layout + Clone, VL: Layout + Clone>(
        &self,
        storage: &GameStorage<L>,
        options: &VanillaLaunchOptions<VL>,
        authorizer: A,
    ) -> Result<LaunchedVanillaVersion<L, VL>>
    where
        A: Authorizer,
    {
        let ready_version = self.ready(storage, &options.version).await?;
        self.launch_ready(ready_version, options, authorizer)
            .await
    }

    pub async fn launch_ready<A, L: Layout + Clone, VL: Layout + Clone>(
        &self,
        ready_version: ReadyVanillaVersion<L, VL>,
        options: &VanillaLaunchOptions<VL>,
        authorizer: A,
    ) -> Result<LaunchedVanillaVersion<L, VL>>
    where
        A: Authorizer,
    {
        let runtime = self
            .runtime_for_ready_version(&ready_version, options.runtime_major_version)
            .await?;
        let child = self
            .build_launch_builder(authorizer, runtime.clone(), &ready_version, options)?
            .launch()
            .await?;

        Ok(LaunchedVanillaVersion {
            ready_version,
            runtime,
            child,
        })
    }

    async fn runtime_for_ready_version<L: Layout, VL: Layout>(
        &self,
        ready_version: &ReadyVanillaVersion<L, VL>,
        runtime_major_version: Option<usize>,
    ) -> Result<Distribution> {
        let required_major_version =
            runtime_major_version.unwrap_or_else(|| ready_version.required_java_major_version());

        Distribution::find_cached_by_java_major(required_major_version)
            .await
            .with_context(|| {
                format!(
                    "can't find a local Java runtime with major version {}",
                    required_major_version
                )
            })
    }

    fn build_launch_builder<A, L: Layout + Clone, VL: Layout + Clone>(
        &self,
        authorizer: A,
        runtime: Distribution,
        ready_version: &ReadyVanillaVersion<L, VL>,
        options: &VanillaLaunchOptions<VL>,
    ) -> Result<LaunchBuilder<A, L, VL>>
    where
        A: Authorizer,
    {
        let mut builder = LaunchBuilder::new(
            authorizer,
            runtime,
            ready_version.resolved_version.version.clone(),
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
            builder = builder.set_quick_play_path(
                quick_play.path.clone(),
                quick_play.multiplayer.clone(),
                quick_play.singleplayer.clone(),
                quick_play.realms.clone(),
            );
        }

        Ok(builder)
    }

    async fn resolve_or_load<L: Layout + Clone, VL: Layout + Clone>(
        &self,
        storage: &GameStorage<L>,
        version: &VanillaVersionSpec<VL>,
    ) -> Result<ResolvedVanillaVersion<L, VL>> {
        if let Ok(local_version) =
            storage.version(version.version_name.clone(), version.version_layout.clone())
        {
            if let Ok(resolved) =
                ResolvedVanillaVersion::load(self.client.baseurl.clone(), local_version)
            {
                return Ok(resolved);
            }
        }

        self.client
            .resolve_vanilla_version(
                storage,
                version.version_id.clone(),
                version.version_name.clone(),
                version.version_layout.clone(),
            )
            .await
    }
}
