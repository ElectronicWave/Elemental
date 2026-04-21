use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use elemental_core::{
    auth::authorizer::Authorizer,
    launcher::command::LaunchCommand,
    runtime::distribution::Distribution,
    storage::{Storage, layout::Layout},
};
use elemental_infra::downloader::core::ElementalDownloader;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::shared::{
        build_version_json_launch_command, installed_version_json_driver,
        launch_version_json_instance, load_prepared_version_json, metadata_contains_library_prefix,
    },
    drivers::vanilla::{
        config::VanillaLaunchConfig,
        prepared::{
            LaunchedVanillaVersion, PreparedVanillaVersion, ResolvedVanillaMetadata,
            ResolvedVanillaVersion,
        },
        source::VanillaSource,
    },
    families::version_json::{PistonMetaData, VersionJsonInstanceLayout, VersionJsonRootLayout},
    inspect::InstanceProbe,
};

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
            ElementalDownloader::with_config_default()
                .context("create default elemental downloader failed")?,
        ))
    }

    pub fn source(&self) -> &VanillaSource {
        &self.source
    }

    pub fn downloader(&self) -> &ElementalDownloader {
        self.downloader.as_ref()
    }

    pub async fn prepare<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        &self,
        instance: &Storage<VL, Storage<L>>,
        version_id: String,
    ) -> Result<PreparedVanillaVersion<L, VL>> {
        let resolved = self.resolve_or_load(instance, version_id).await?;
        resolved.prepare(self.downloader()).await
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
        version_id: String,
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
        version_id: String,
    ) -> Result<ResolvedVanillaVersion<L, VL>> {
        self.resolve_metadata(version_id)
            .await?
            .persist(instance)
            .await
    }

    async fn resolve_metadata(&self, version_id: String) -> Result<ResolvedVanillaMetadata> {
        let launchmeta = self.source.launch_meta().await?;
        let metadata_url = launchmeta
            .versions
            .iter()
            .find(|version| version.id == version_id)
            .context(format!("Can't find version named `{version_id}`"))?
            .url
            .clone();
        let metadata = self.source.piston_meta(metadata_url).await?;
        let asset_index_objects = self
            .source
            .asset_index_objects(&metadata.asset_index.url)
            .await?;

        Ok(ResolvedVanillaMetadata::new(
            self.source.endpoints().clone(),
            metadata,
            asset_index_objects,
        ))
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

        Ok(Some(installed_version_json_driver(
            metadata,
            <Self as Driver<L, VL>>::descriptor(self),
            None,
        )))
    }
}

fn has_loader_marker(metadata: &PistonMetaData) -> bool {
    metadata_contains_library_prefix(
        metadata,
        &[
            "net.minecraftforge:forge:",
            "net.neoforged:forge:",
            "net.neoforged:neoforge:",
            "net.fabricmc:fabric-loader:",
        ],
    )
}
