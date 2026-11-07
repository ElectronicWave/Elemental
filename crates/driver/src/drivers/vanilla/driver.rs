use crate::error::Result;
use async_trait::async_trait;
use elemental_core::{
    minecraft::MinecraftVersionId,
    storage::{Storage, layout::Layout},
};
use elemental_infra::downloader::DownloadSession;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    descriptors::VANILLA_DRIVER,
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::vanilla::{
        prepared::ResolvedVanillaVersion,
        source::{VanillaSource, resolve_vanilla_metadata},
    },
    families::version_json::{
        PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
    },
    inspect::{InstanceProbe, LibraryPrefixSet},
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

    pub async fn prepare<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        session: &DownloadSession,
        instance: &Storage<VL, Storage<L>>,
        version_id: MinecraftVersionId,
    ) -> Result<PreparedVersionJsonInstance<L, VL>> {
        self.resolve_or_load(instance, version_id)
            .await?
            .prepare(session)
            .await
    }

    pub async fn load_prepared<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedVersionJsonInstance<L, VL>> {
        ResolvedVanillaVersion::load(self.source.endpoints().clone(), instance.clone())?
            .into_prepared()
            .await
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

#[async_trait]
impl<L: Layout, VL: Layout> Driver<L, VL> for VanillaDriver {
    fn descriptor(&self) -> DriverDescriptor {
        VANILLA_DRIVER
    }

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };
        if has_loader_marker(metadata) {
            return Ok(None);
        }

        Ok(Some(InstalledDriver::version_json(
            <Self as Driver<L, VL>>::descriptor(self),
            metadata,
            None,
        )))
    }
}

fn has_loader_marker(metadata: &PistonMetaData) -> bool {
    LOADER_MARKERS.matches(metadata)
}
