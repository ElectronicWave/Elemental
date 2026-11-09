use std::marker::PhantomData;

use crate::error::Result;
use async_trait::async_trait;
use elemental_core::storage::{Storage, layout::Layout};

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver, PrepareRequest},
    drivers::vanilla::source::VanillaSource,
    families::{
        installer::{
            InstallerFamily, InstallerFamilyRemoteResolver, ResolvedInstallerFamilyVersion,
        },
        version_json::{
            PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
        },
    },
    inspect::InstanceProbe,
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
    pub fn new(source: F::Source) -> Self {
        Self {
            source,
            vanilla_source: VanillaSource::default(),
            family: PhantomData,
        }
    }

    pub fn with_vanilla_source(mut self, vanilla_source: VanillaSource) -> Self {
        self.vanilla_source = vanilla_source;
        self
    }

    pub fn descriptor(&self) -> DriverDescriptor {
        F::descriptor()
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
    pub fn with_defaults() -> Self {
        Self::new(F::Source::default())
    }
}

#[async_trait(?Send)]
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

    async fn inspect(&self, probe: &InstanceProbe) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };

        Ok(F::IDENTITY.inspect_installed(metadata))
    }

    async fn prepare(
        &self,
        request: &PrepareRequest<'_, L, VL>,
    ) -> Result<PreparedVersionJsonInstance<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        let loader_version = request.required_loader_version(self.descriptor())?;
        ResolvedInstallerFamilyVersion::new(
            self.source.clone(),
            request.instance.clone(),
            request.game_version.clone(),
            loader_version.clone(),
        )?
        .prepare(
            request.session,
            &self.vanilla_source,
            &self.remote_resolver(),
            request.runtime_options,
            request.runtime_resolver,
        )
        .await
    }

    async fn load_prepared(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedVersionJsonInstance<L, VL>>
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
}
