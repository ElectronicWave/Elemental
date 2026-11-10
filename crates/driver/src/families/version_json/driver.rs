use crate::error::Result;
use async_trait::async_trait;
use elemental_core::storage::{Storage, layout::Layout};

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver, PrepareRequest},
    drivers::vanilla::source::VanillaSource,
    inspect::InstanceProbe,
};

use super::{
    PreparedVersionJsonInstance, ResolvedVersionJsonInstance, VersionJsonInstanceLayout,
    VersionJsonRootLayout, family::ProfiledVersionJsonFamily,
};

mod resolve;

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

    fn remote_resolver(&self) -> F::RemoteResolver {
        self.family
            .remote_resolver(&self.vanilla_source, &self.source)
    }
}

#[async_trait(?Send)]
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

    async fn inspect(&self, probe: &InstanceProbe) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };

        Ok(self.family.inspect_installed(metadata))
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
        let (resolved, download_status) = self
            .resolve_or_load(
                request.instance,
                request.game_version.clone(),
                loader_version.clone(),
            )
            .await?;
        resolved
            .prepare_with_download_status(request.session, download_status)
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
        ResolvedVersionJsonInstance::load(self.remote_resolver(), instance.clone())?
            .finish_preparation()
            .await
    }
}
