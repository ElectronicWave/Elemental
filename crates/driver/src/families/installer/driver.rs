use async_trait::async_trait;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver, PrepareRequest, ResolvedDriverPlan},
    drivers::vanilla::source::VanillaSource,
    error::Result,
    families::{
        installer::{
            InstallerFamilyRemoteResolver, InstallerMavenSource, MavenInstallerSpec,
            ResolvedInstallerFamilyVersion,
        },
        version_json::{InstanceStorage, PreparedVersionJsonInstance},
    },
    inspect::InstanceProbe,
};

#[derive(Debug, Clone)]
pub struct InstallerFamilyDriver {
    source: InstallerMavenSource,
    vanilla_source: VanillaSource,
}

impl InstallerFamilyDriver {
    pub fn new(source: InstallerMavenSource) -> Self {
        Self {
            source,
            vanilla_source: VanillaSource::default(),
        }
    }

    pub fn for_spec(spec: &'static MavenInstallerSpec) -> Self {
        Self::new(InstallerMavenSource::official(spec))
    }

    pub fn with_vanilla_source(mut self, vanilla_source: VanillaSource) -> Self {
        self.vanilla_source = vanilla_source;
        self
    }

    pub fn descriptor(&self) -> DriverDescriptor {
        self.source.spec().descriptor()
    }

    fn remote_resolver(&self) -> InstallerFamilyRemoteResolver {
        InstallerFamilyRemoteResolver::new(
            self.vanilla_source.endpoints().clone(),
            self.source.endpoints().clone(),
        )
    }
}

#[async_trait]
impl Driver for InstallerFamilyDriver {
    fn descriptor(&self) -> DriverDescriptor {
        self.source.spec().descriptor()
    }

    async fn inspect(&self, probe: &InstanceProbe) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };

        Ok(self.source.spec().identity.inspect_installed(metadata))
    }

    async fn resolve(&self, request: &PrepareRequest<'_>) -> Result<ResolvedDriverPlan> {
        let loader_version = request.required_loader_version(self.descriptor())?;
        ResolvedInstallerFamilyVersion::new(
            self.source.clone(),
            request.instance.clone(),
            request.game_version.clone(),
            loader_version.clone(),
        )?
        .resolve(
            request.session,
            &self.vanilla_source,
            self.remote_resolver(),
        )
        .await
    }

    async fn verify_prepared(
        &self,
        instance: &InstanceStorage,
        prepared: &PreparedVersionJsonInstance,
    ) -> Result<()> {
        ResolvedInstallerFamilyVersion::verify_prepared(&self.source, instance, prepared).await
    }
}
