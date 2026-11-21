use crate::error::Result;
use async_trait::async_trait;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver, PrepareRequest, ResolvedDriverPlan},
    drivers::vanilla::source::VanillaSource,
    inspect::InstanceProbe,
};

use super::family::ProfiledVersionJsonFamily;

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

#[async_trait]
impl<F> Driver for ProfiledVersionJsonDriver<F>
where
    F: ProfiledVersionJsonFamily,
    F::Source: Sync,
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
