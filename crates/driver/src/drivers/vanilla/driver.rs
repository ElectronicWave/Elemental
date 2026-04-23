use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use elemental_core::{
    auth::authorizer::Authorizer,
    launcher::command::LaunchCommand,
    minecraft::MinecraftVersionId,
    runtime::distribution::Distribution,
    storage::{Storage, layout::Layout},
};
use elemental_infra::downloader::core::ElementalDownloader;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::vanilla::{
        config::VanillaLaunchConfig,
        prepared::{
            LaunchedVanillaVersion, PreparedVanillaVersion, ResolvedVanillaMetadata,
            ResolvedVanillaVersion,
        },
        source::{VanillaSource, resolve_vanilla_metadata},
    },
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonRootLayout, build_version_json_launch_command,
        launch_version_json_instance, load_prepared_version_json, persist_version_json,
        prepare_version_json,
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
    downloader: Arc<ElementalDownloader>,
}

impl VanillaDriver {
    pub fn new(source: VanillaSource, downloader: Arc<ElementalDownloader>) -> Self {
        Self { source, downloader }
    }

    pub fn with_defaults() -> Result<Self> {
        Ok(Self::new(
            VanillaSource::default(),
            ElementalDownloader::new(),
        ))
    }

    pub async fn prepare<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
        version_id: MinecraftVersionId,
    ) -> Result<PreparedVanillaVersion<L, VL>> {
        prepare_version_json(self.downloader.as_ref(), || {
            self.resolve_or_load(instance, version_id)
        })
        .await
    }

    pub async fn load_prepared<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedVanillaVersion<L, VL>> {
        load_prepared_version_json(self.source.endpoints().clone(), instance).await
    }

    pub async fn launch<
        A,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        prepared_version: PreparedVanillaVersion<L, VL>,
        config: &VanillaLaunchConfig,
        authorizer: A,
    ) -> Result<LaunchedVanillaVersion<L, VL>>
    where
        A: Authorizer,
    {
        launch_version_json_instance(authorizer, prepared_version, config).await
    }

    pub async fn build_launch_command<
        A,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        authorizer: A,
        prepared_version: &PreparedVanillaVersion<L, VL>,
        config: &VanillaLaunchConfig,
    ) -> Result<(Distribution, LaunchCommand)>
    where
        A: Authorizer,
    {
        build_version_json_launch_command(authorizer, prepared_version, config).await
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

        self.resolve_version(instance, version_id).await
    }

    async fn resolve_version<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
        version_id: MinecraftVersionId,
    ) -> Result<ResolvedVanillaVersion<L, VL>> {
        persist_version_json(instance, || self.resolve_metadata(version_id)).await
    }

    async fn resolve_metadata(
        &self,
        version_id: MinecraftVersionId,
    ) -> Result<ResolvedVanillaMetadata> {
        resolve_vanilla_metadata(&self.source, version_id.as_str()).await
    }
}

#[async_trait]
impl<L: Layout, VL: Layout> Driver<L, VL> for VanillaDriver {
    fn descriptor(&self) -> DriverDescriptor {
        DriverDescriptor {
            id: "vanilla",
            name: "Vanilla",
        }
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
