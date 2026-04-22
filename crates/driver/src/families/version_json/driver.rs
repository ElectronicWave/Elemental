use std::{fmt::Debug, sync::Arc};

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
use elemental_schema::fabric::ProfileJson;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::vanilla::source::{VanillaSource, resolve_vanilla_metadata},
    families::version_json::{
        LaunchedVersionJsonInstance, PASSTHROUGH_PROFILE_BEHAVIOR, PreparedVersionJsonInstance,
        ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata, UpstreamUrlRewriter,
        VanillaFallbackRemoteResolver, VersionJsonInstanceLayout, VersionJsonLaunchConfig,
        VersionJsonRemoteResolver, VersionJsonRootLayout, build_version_json_launch_command,
        launch_version_json_instance, load_prepared_version_json, merge_profile_with_behavior,
        persist_version_json, prepare_version_json,
    },
    inspect::{InstanceProbe, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
};

#[async_trait(?Send)]
pub trait ProfiledVersionJsonFamily: Clone + Debug + Send + Sync + 'static {
    type Source: Clone + Debug + Send + Sync + 'static;
    type Profile: Send;
    type RemoteResolver: VersionJsonRemoteResolver + Clone + Send + Sync + 'static;

    fn descriptor(&self) -> DriverDescriptor;

    fn default_source(&self) -> Result<Self::Source>;

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver;

    async fn profile(
        &self,
        source: &Self::Source,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<Self::Profile>;

    fn merge_profile(
        &self,
        base_metadata: PistonMetaData,
        profile: Self::Profile,
    ) -> Result<PistonMetaData>;

    fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool;

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver>;
}

pub trait ProfiledVersionJsonFamilyExt: ProfiledVersionJsonFamily + Sized {
    fn build_driver(
        self,
        source: Self::Source,
        vanilla_source: VanillaSource,
        downloader: Arc<ElementalDownloader>,
    ) -> ProfiledVersionJsonDriver<Self> {
        ProfiledVersionJsonDriver::new(self, source, vanilla_source, downloader)
    }

    fn build_driver_with_defaults(self) -> Result<ProfiledVersionJsonDriver<Self>> {
        ProfiledVersionJsonDriver::with_defaults(self)
    }
}

impl<F> ProfiledVersionJsonFamilyExt for F where F: ProfiledVersionJsonFamily {}

pub fn default_profiled_source<S>() -> Result<S>
where
    S: Default,
{
    Ok(S::default())
}

pub fn vanilla_fallback_remote_resolver<E>(
    family_name: &'static str,
    vanilla_source: &VanillaSource,
    family_endpoints: &E,
) -> VanillaFallbackRemoteResolver<E>
where
    E: UpstreamUrlRewriter,
{
    VanillaFallbackRemoteResolver::new(
        family_name,
        vanilla_source.endpoints().clone(),
        family_endpoints.clone(),
    )
}

#[async_trait(?Send)]
pub trait PassthroughProfiledVersionJsonFamily:
    Clone + std::fmt::Debug + Send + Sync + 'static
{
    type Source: Default + Clone + std::fmt::Debug + Send + Sync + 'static;
    type Endpoints: UpstreamUrlRewriter;

    const DRIVER: DriverDescriptor;
    const FAMILY_NAME: &'static str;
    const IDENTITY: ProfiledDriverIdentity;

    fn source_endpoints(source: &Self::Source) -> &Self::Endpoints;

    async fn profile(
        &self,
        source: &Self::Source,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<ProfileJson>;
}

#[async_trait(?Send)]
impl<F> ProfiledVersionJsonFamily for F
where
    F: PassthroughProfiledVersionJsonFamily,
{
    type Source = F::Source;
    type Profile = ProfileJson;
    type RemoteResolver = VanillaFallbackRemoteResolver<F::Endpoints>;

    fn descriptor(&self) -> DriverDescriptor {
        F::DRIVER
    }

    fn default_source(&self) -> Result<Self::Source> {
        default_profiled_source()
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver {
        vanilla_fallback_remote_resolver(
            F::FAMILY_NAME,
            vanilla_source,
            F::source_endpoints(source),
        )
    }

    async fn profile(
        &self,
        source: &Self::Source,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<Self::Profile> {
        PassthroughProfiledVersionJsonFamily::profile(self, source, game_version, loader_version)
            .await
    }

    fn merge_profile(
        &self,
        base_metadata: PistonMetaData,
        profile: Self::Profile,
    ) -> Result<PistonMetaData> {
        merge_profile_with_behavior(&PASSTHROUGH_PROFILE_BEHAVIOR, base_metadata, profile)
    }

    fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool {
        F::IDENTITY.local_metadata_needs_refresh(metadata, game_version, loader_version.as_str())
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        F::IDENTITY.inspect_installed(metadata)
    }
}

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

    pub fn family(&self) -> &F {
        &self.family
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
    ) -> Result<PreparedVersionJsonInstance<F::RemoteResolver, L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        prepare_version_json(self.downloader(), || {
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
            .remote_resolver(self.vanilla_source(), self.source())
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
            resolve_vanilla_metadata(self.vanilla_source(), game_version.as_str()).await?;
        let profile = self
            .family
            .profile(self.source(), &game_version, &loader_version)
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
