use crate::error::Result;
use crate::{
    descriptors::VANILLA_DRIVER,
    driver::{Driver, DriverDescriptor, InstalledDriver, PrepareRequest},
    drivers::vanilla::{
        prepared::ResolvedVanillaVersion,
        source::{VanillaSource, resolve_vanilla_metadata},
    },
    families::version_json::{
        PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
    },
    inspect::{InstanceProbe, LibraryPrefixSet},
};
use async_trait::async_trait;
use elemental_core::{
    minecraft::MinecraftVersionId,
    storage::{Storage, layout::Layout},
};

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

    async fn resolve_or_load<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
        version_id: MinecraftVersionId,
    ) -> Result<ResolvedVanillaVersion<L, VL>> {
        if let Ok(resolved) =
            ResolvedVanillaVersion::load(self.source.endpoints().clone(), instance.clone())
        {
            return Ok(resolved);
        }

        resolve_vanilla_metadata(&self.source, version_id.as_str())
            .await?
            .persist(instance)
            .await
    }
}

#[async_trait(?Send)]
impl<L: Layout, VL: Layout> Driver<L, VL> for VanillaDriver {
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
            <Self as Driver<L, VL>>::descriptor(self),
            metadata,
            None,
        )))
    }

    async fn prepare(
        &self,
        request: &PrepareRequest<'_, L, VL>,
    ) -> Result<PreparedVersionJsonInstance<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        self.resolve_or_load(request.instance, request.game_version.clone())
            .await?
            .prepare(request.session)
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
        ResolvedVanillaVersion::load(self.source.endpoints().clone(), instance.clone())?
            .finish_preparation()
            .await
    }
}
