use crate::error::Result;
use crate::{
    descriptors::VANILLA_DRIVER,
    driver::{
        Driver, DriverDescriptor, InstalledDriver, InstanceProbe, PrepareRequest,
        ResolvedDriverPlan,
    },
    drivers::vanilla::{
        ResolvedVanillaVersion,
        source::{VanillaSource, resolve_vanilla_metadata},
    },
    families::version_json::InstanceStorage,
    inspect::LibraryPrefixSet,
};
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

const LOADER_MARKERS: LibraryPrefixSet = LibraryPrefixSet::new(&[
    "net.minecraftforge:forge:",
    "net.neoforged:forge:",
    "net.neoforged:neoforge:",
    "net.fabricmc:fabric-loader:",
]);

pub struct VanillaDriver {
    source: VanillaSource,
}

impl VanillaDriver {
    pub fn new(source: VanillaSource) -> Self {
        Self { source }
    }

    pub fn with_defaults() -> Self {
        Self::new(VanillaSource::default())
    }

    async fn resolve_or_load(
        &self,
        instance: &InstanceStorage,
        version_id: MinecraftVersionId,
    ) -> Result<ResolvedVanillaVersion> {
        match ResolvedVanillaVersion::load(instance.clone()) {
            Ok(resolved) => return Ok(resolved),
            Err(error) if error.is_not_found() => {}
            Err(error) => return Err(error),
        }

        resolve_vanilla_metadata(&self.source, version_id.as_str())
            .await?
            .persist(instance)
            .await
    }
}

#[async_trait]
impl Driver for VanillaDriver {
    fn descriptor(&self) -> DriverDescriptor {
        VANILLA_DRIVER
    }

    async fn inspect(&self, probe: &InstanceProbe) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };
        if LOADER_MARKERS.matches(metadata) {
            return Ok(None);
        }

        Ok(Some(InstalledDriver::version_json(
            self.descriptor(),
            metadata,
            None,
        )))
    }

    async fn resolve(&self, request: &PrepareRequest<'_>) -> Result<ResolvedDriverPlan> {
        let instance = self
            .resolve_or_load(request.instance, request.game_version.clone())
            .await?;
        let download_status = instance.download_status().ok();
        Ok(ResolvedDriverPlan {
            instance,
            remote_resolver: Box::new(self.source.endpoints().clone()),
            download_status,
            installer: None,
        })
    }
}
