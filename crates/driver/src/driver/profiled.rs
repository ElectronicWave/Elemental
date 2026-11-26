use std::fmt::Debug;

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::{fabric::ProfileJson, mojang::piston::PistonMetaData};

use crate::{
    descriptors::DriverDescriptor,
    error::Result,
    families::version_json::{
        InstanceStorage, PASSTHROUGH_PROFILE_BEHAVIOR, ResolvedVersionJsonInstance,
        ResolvedVersionJsonMetadata, VanillaSource, VersionJsonDownloadStatus,
        VersionJsonRemoteResolver, merge_profile_with_behavior, resolve_vanilla_metadata,
    },
    inspect::{InstalledDriver, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
};

use super::{Driver, InstanceProbe, PrepareRequest, ResolvedDriverPlan};

#[async_trait]
pub trait ProfiledVersionJsonFamily: Clone + Debug + Send + Sync + 'static {
    type Source: Clone + Debug + Send + Sync + 'static;
    type RemoteResolver: VersionJsonRemoteResolver + Clone + Send + Sync + 'static;

    fn descriptor(&self) -> DriverDescriptor;

    fn identity(&self) -> Option<ProfiledDriverIdentity> {
        None
    }

    fn default_source(&self) -> Result<Self::Source>
    where
        Self::Source: Default,
    {
        Ok(Self::Source::default())
    }

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
    ) -> Result<ProfileJson>;

    fn merge_profile(
        &self,
        base_metadata: PistonMetaData,
        profile: ProfileJson,
    ) -> Result<PistonMetaData> {
        merge_profile_with_behavior(&PASSTHROUGH_PROFILE_BEHAVIOR, base_metadata, profile)
    }

    fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool {
        self.identity().is_some_and(|identity| {
            identity.local_metadata_needs_refresh(metadata, game_version, loader_version.as_str())
        })
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        self.identity()
            .and_then(|identity| identity.inspect_installed(metadata))
    }
}

pub trait ProfiledVersionJsonFamilyExt: ProfiledVersionJsonFamily + Sized {
    fn build_driver(self, source: Self::Source) -> ProfiledVersionJsonDriver<Self> {
        ProfiledVersionJsonDriver::new(self, source)
    }

    fn build_driver_with_defaults(self) -> Result<ProfiledVersionJsonDriver<Self>>
    where
        Self::Source: Default,
    {
        ProfiledVersionJsonDriver::with_defaults(self)
    }
}

impl<F> ProfiledVersionJsonFamilyExt for F where F: ProfiledVersionJsonFamily {}

#[derive(Debug, Clone)]
pub struct ProfiledVersionJsonDriver<F>
where
    F: ProfiledVersionJsonFamily,
{
    family: F,
    source: F::Source,
    vanilla_source: VanillaSource,
}

impl<F> ProfiledVersionJsonDriver<F>
where
    F: ProfiledVersionJsonFamily,
{
    pub fn new(family: F, source: F::Source) -> Self {
        Self {
            family,
            source,
            vanilla_source: VanillaSource::default(),
        }
    }

    pub fn with_vanilla_source(mut self, vanilla_source: VanillaSource) -> Self {
        self.vanilla_source = vanilla_source;
        self
    }

    pub fn with_defaults(family: F) -> Result<Self>
    where
        F::Source: Default,
    {
        let source = family.default_source()?;
        Ok(Self::new(family, source))
    }

    pub fn descriptor(&self) -> DriverDescriptor {
        self.family.descriptor()
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        self.family.inspect_installed(metadata)
    }

    fn remote_resolver(&self) -> F::RemoteResolver {
        self.family
            .remote_resolver(&self.vanilla_source, &self.source)
    }

    async fn resolve_or_load(
        &self,
        instance: &InstanceStorage,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<(
        ResolvedVersionJsonInstance,
        Option<VersionJsonDownloadStatus>,
    )> {
        if let Some((resolved, status)) = try_load_downloaded_instance(instance)?
            && !self.family.local_metadata_needs_refresh(
                &resolved.metadata,
                &game_version,
                &loader_version,
            )
        {
            return Ok((resolved, Some(status)));
        }

        let base_metadata =
            resolve_vanilla_metadata(&self.vanilla_source, game_version.as_str()).await?;
        let profile = self
            .family
            .profile(&self.source, &game_version, &loader_version)
            .await?;
        let metadata = self.family.merge_profile(base_metadata.metadata, profile)?;
        let resolved =
            ResolvedVersionJsonMetadata::new(metadata, base_metadata.asset_index_objects)
                .persist(instance)
                .await?;
        Ok((resolved, None))
    }
}

#[async_trait]
impl<F> Driver for ProfiledVersionJsonDriver<F>
where
    F: ProfiledVersionJsonFamily,
    F::Source: Sync,
{
    fn descriptor(&self) -> DriverDescriptor {
        ProfiledVersionJsonDriver::descriptor(self)
    }

    async fn inspect(&self, probe: &InstanceProbe) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };

        Ok(self.inspect_installed(metadata))
    }

    async fn resolve(&self, request: &PrepareRequest<'_>) -> Result<ResolvedDriverPlan> {
        let loader_version = request.required_loader_version(self.descriptor())?;
        let (instance, download_status) = self
            .resolve_or_load(
                request.instance,
                request.game_version.clone(),
                loader_version.clone(),
            )
            .await?;
        Ok(ResolvedDriverPlan {
            instance,
            remote_resolver: Box::new(self.remote_resolver()),
            download_status,
            installer: None,
        })
    }
}

fn try_load_downloaded_instance(
    instance: &InstanceStorage,
) -> Result<Option<(ResolvedVersionJsonInstance, VersionJsonDownloadStatus)>> {
    match ResolvedVersionJsonInstance::load(instance.clone()) {
        Ok(resolved) => {
            let status = resolved.download_status()?;
            if status.is_downloaded() {
                return Ok(Some((resolved, status)));
            }
            Ok(None)
        }
        Err(error) if !error.is_not_found() => Err(error),
        Err(_) => Ok(None),
    }
}
