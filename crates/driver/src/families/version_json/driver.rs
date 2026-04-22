use std::sync::Arc;

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
    drivers::vanilla::source::{VanillaSource, resolve_vanilla_metadata},
    inspect::InstanceProbe,
    loader_version::LoaderVersionId,
};

use super::{
    LaunchedVersionJsonInstance, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
    ResolvedVersionJsonMetadata, VersionJsonInstanceLayout, VersionJsonLaunchConfig,
    VersionJsonRootLayout, build_version_json_launch_command, family::ProfiledVersionJsonFamily,
    launch_version_json_instance, load_prepared_version_json, persist_version_json,
    prepare_version_json,
};

#[derive(Debug, Clone)]
pub struct ProfiledVersionJsonDriver<F>
where
    F: ProfiledVersionJsonFamily,
{
    family: F,
    source: F::Source,
    vanilla_source: VanillaSource,
    downloader: Arc<ElementalDownloader>,
}

impl<F> ProfiledVersionJsonDriver<F>
where
    F: ProfiledVersionJsonFamily,
{
    pub fn new(
        family: F,
        source: F::Source,
        vanilla_source: VanillaSource,
        downloader: Arc<ElementalDownloader>,
    ) -> Self {
        Self {
            family,
            source,
            vanilla_source,
            downloader,
        }
    }

    pub fn with_defaults(family: F) -> Result<Self> {
        Ok(Self::new(
            family.clone(),
            family.default_source()?,
            VanillaSource::default(),
            ElementalDownloader::with_config_default()
                .context("create default elemental downloader failed")?,
        ))
    }

    pub async fn prepare<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<PreparedVersionJsonInstance<F::RemoteResolver, L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        prepare_version_json(self.downloader.as_ref(), || {
            self.resolve_or_load(instance, game_version, loader_version)
        })
        .await
    }

    pub async fn load_prepared<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedVersionJsonInstance<F::RemoteResolver, L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        load_prepared_version_json(self.remote_resolver(), instance).await
    }

    pub async fn launch<A, L, VL>(
        &self,
        prepared_version: PreparedVersionJsonInstance<F::RemoteResolver, L, VL>,
        config: &VersionJsonLaunchConfig,
        authorizer: A,
    ) -> Result<LaunchedVersionJsonInstance<F::RemoteResolver, L, VL>>
    where
        A: Authorizer,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        launch_version_json_instance(authorizer, prepared_version, config).await
    }

    pub async fn build_launch_command<A, L, VL>(
        &self,
        authorizer: A,
        prepared_version: &PreparedVersionJsonInstance<F::RemoteResolver, L, VL>,
        config: &VersionJsonLaunchConfig,
    ) -> Result<(Distribution, LaunchCommand)>
    where
        A: Authorizer,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        build_version_json_launch_command(authorizer, prepared_version, config).await
    }

    fn remote_resolver(&self) -> F::RemoteResolver {
        self.family
            .remote_resolver(&self.vanilla_source, &self.source)
    }

    async fn resolve_or_load<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<ResolvedVersionJsonInstance<F::RemoteResolver, L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        if let Ok(resolved) =
            ResolvedVersionJsonInstance::load(self.remote_resolver(), instance.clone())
        {
            let status = resolved.status().await?;
            if status.is_downloaded()
                && !self.family.local_metadata_needs_refresh(
                    &resolved.metadata,
                    &game_version,
                    &loader_version,
                )
            {
                return Ok(resolved);
            }
        }

        self.resolve_version(instance, game_version, loader_version)
            .await
    }

    async fn resolve_version<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<ResolvedVersionJsonInstance<F::RemoteResolver, L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        persist_version_json(instance, || {
            self.resolve_metadata(game_version, loader_version)
        })
        .await
    }

    async fn resolve_metadata(
        &self,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<ResolvedVersionJsonMetadata<F::RemoteResolver>> {
        let base_metadata =
            resolve_vanilla_metadata(&self.vanilla_source, game_version.as_str()).await?;
        let profile = self
            .family
            .profile(&self.source, &game_version, &loader_version)
            .await?;
        let metadata = self.family.merge_profile(base_metadata.metadata, profile)?;

        Ok(ResolvedVersionJsonMetadata::new(
            self.remote_resolver(),
            metadata,
            base_metadata.asset_index_objects,
        ))
    }
}

#[async_trait]
impl<F, L, VL> Driver<L, VL> for ProfiledVersionJsonDriver<F>
where
    F: ProfiledVersionJsonFamily,
    F::Source: Sync,
    L: Layout,
    VL: Layout,
{
    fn descriptor(&self) -> DriverDescriptor {
        self.family.descriptor()
    }

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };

        Ok(self.family.inspect_installed(metadata))
    }
}
