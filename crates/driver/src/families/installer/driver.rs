use std::marker::PhantomData;

use crate::error::Result;
use async_trait::async_trait;
use elemental_core::{
    minecraft::MinecraftVersionId,
    storage::{Storage, layout::Layout},
};
use elemental_infra::downloader::DownloadSession;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::vanilla::source::VanillaSource,
    families::{
        installer::{
            InstallerFamily, InstallerFamilyRemoteResolver, PreparedInstallerVersion,
            ResolvedInstallerFamilyVersion,
        },
        version_json::{VersionJsonInstanceLayout, VersionJsonLaunchConfig, VersionJsonRootLayout},
    },
    inspect::InstanceProbe,
    loader_version::LoaderVersionId,
};

#[derive(Debug, Clone)]
pub struct InstallerFamilyDriver<F>
where
    F: InstallerFamily,
{
    source: F::Source,
    vanilla_source: VanillaSource,
    family: PhantomData<F>,
}

impl<F> InstallerFamilyDriver<F>
where
    F: InstallerFamily,
{
    pub fn new(source: F::Source, vanilla_source: VanillaSource) -> Self {
        Self {
            source,
            vanilla_source,
            family: PhantomData,
        }
    }

    pub fn descriptor(&self) -> DriverDescriptor {
        F::descriptor()
    }

    pub async fn prepare<L, VL>(
        &self,
        session: &DownloadSession,
        instance: &Storage<VL, Storage<L>>,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<PreparedInstallerVersion<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        self.prepare_with_config(
            session,
            instance,
            game_version,
            loader_version,
            &VersionJsonLaunchConfig::new(),
        )
        .await
    }

    pub async fn prepare_with_config<L, VL>(
        &self,
        session: &DownloadSession,
        instance: &Storage<VL, Storage<L>>,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
        config: &VersionJsonLaunchConfig,
    ) -> Result<PreparedInstallerVersion<L, VL>>
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
            session,
            &self.vanilla_source,
            &self.remote_resolver(),
            config.runtime_executable_path.as_deref(),
            config.runtime_validation,
        )
        .await
    }

    pub async fn load_prepared<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedInstallerVersion<L, VL>>
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

    fn remote_resolver(&self) -> InstallerFamilyRemoteResolver<F> {
        InstallerFamilyRemoteResolver::new(
            self.vanilla_source.endpoints().clone(),
            F::source_endpoints(&self.source).clone(),
        )
    }
}

impl<F> InstallerFamilyDriver<F>
where
    F: InstallerFamily,
    F::Source: Default,
{
    pub fn with_defaults() -> Result<Self> {
        Ok(Self::new(F::Source::default(), VanillaSource::default()))
    }
}

#[async_trait]
impl<F, L, VL> Driver<L, VL> for InstallerFamilyDriver<F>
where
    F: InstallerFamily,
    F::Source: Sync,
    L: Layout,
    VL: Layout,
{
    fn descriptor(&self) -> DriverDescriptor {
        F::descriptor()
    }

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };

        Ok(F::IDENTITY.inspect_installed(metadata))
    }
}
