use async_trait::async_trait;
use elemental_core::storage::{Storage, layout::Layout};

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver, PrepareRequest},
    drivers::vanilla::source::VanillaSource,
    error::Result,
    families::{
        installer::{
            InstallerFamilyRemoteResolver, InstallerMavenSource, MavenInstallerSpec,
            ResolvedInstallerFamilyVersion,
        },
        version_json::{
            PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
        },
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
impl<L, VL> Driver<L, VL> for InstallerFamilyDriver
where
    L: Layout,
    VL: Layout,
{
    fn descriptor(&self) -> DriverDescriptor {
        self.source.spec().descriptor()
    }

    async fn inspect(&self, probe: &InstanceProbe) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };

        Ok(self.source.spec().identity.inspect_installed(metadata))
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
        ResolvedInstallerFamilyVersion::load(&self.source, instance.clone()).await
    }
}
