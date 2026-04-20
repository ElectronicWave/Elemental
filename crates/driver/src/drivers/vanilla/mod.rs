use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use elemental_core::{
    auth::authorizer::Authorizer,
    launcher::{command::LaunchCommand, process},
    runtime::distribution::Distribution,
    storage::{Storage, layout::Layout},
};
use elemental_infra::downloader::{
    core::ElementalDownloader,
    plan::DownloadPlanner,
    task::{DownloadPlan, DownloadTask},
};

use crate::{
    catalog::{Catalog, GameVersions, Release, ReleaseInfo},
    driver::{Driver, DriverDescriptor, InstalledDriver},
    drivers::version_json::{
        builder::MojangLaunchBuilder,
        extensions::PistonMetaLibrariesExt,
        meta::{
            PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaLibraries,
            PistonMetaLibrariesDownloadsArtifact,
        },
        resource::Resource,
        rules::MojangRuleContext,
        storage::{VersionJsonGameStorageExt, VersionJsonVersionStorageExt},
    },
    inspect::InstanceProbe,
};

mod source;

pub use source::{VanillaEndpoints, VanillaSource};

#[derive(Clone)]
pub struct LaunchResolution {
    pub width: String,
    pub height: String,
}

#[derive(Clone)]
pub struct QuickPlayOptions {
    pub path: Option<String>,
    pub multiplayer: Option<String>,
    pub singleplayer: Option<String>,
    pub realms: Option<String>,
}

#[derive(Clone)]
pub struct VanillaLaunchConfig {
    pub runtime_major_version: Option<usize>,
    pub launcher_name: Option<String>,
    pub launcher_version: Option<String>,
    pub client_id: Option<String>,
    pub resolution: Option<LaunchResolution>,
    pub quick_play: Option<QuickPlayOptions>,
}

pub struct VanillaDriver {
    source: VanillaSource,
    downloader: Arc<ElementalDownloader>,
}

pub struct VanillaCatalog {
    source: VanillaSource,
}

#[derive(Debug, Clone)]
pub struct ResolvedVanillaMetadata {
    endpoints: VanillaEndpoints,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaInstallStatus {
    pub metadata_persisted: bool,
    pub asset_index_persisted: bool,
    pub version_artifacts_ready: bool,
    pub assets_ready: bool,
    pub natives_extracted: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedVanillaVersion<L: Layout<Resource = Resource>, VL: Layout> {
    endpoints: VanillaEndpoints,
    pub version: Storage<VL, Storage<L>>,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone)]
pub struct PreparedVanillaVersion<L: Layout<Resource = Resource>, VL: Layout> {
    pub resolved_version: ResolvedVanillaVersion<L, VL>,
    pub install_status: VanillaInstallStatus,
}

pub struct LaunchedVanillaVersion<L: Layout<Resource = Resource>, VL: Layout> {
    pub prepared_version: PreparedVanillaVersion<L, VL>,
    pub runtime: Distribution,
    pub child: tokio::process::Child,
}

pub struct VanillaInstallPlanner<'a, L: Layout<Resource = Resource>, VL: Layout> {
    version: &'a Storage<VL, Storage<L>>,
    metadata: &'a PistonMetaData,
    asset_index_objects: &'a PistonMetaAssetIndexObjects,
    endpoints: VanillaEndpoints,
    rule_context: MojangRuleContext,
}

pub struct VanillaRelease {
    pub version_id: String,
    pub description: Option<String>,
}

impl LaunchResolution {
    pub fn new(width: String, height: String) -> Self {
        Self { width, height }
    }
}

impl QuickPlayOptions {
    pub fn new(
        path: Option<String>,
        multiplayer: Option<String>,
        singleplayer: Option<String>,
        realms: Option<String>,
    ) -> Self {
        Self {
            path,
            multiplayer,
            singleplayer,
            realms,
        }
    }
}

impl VanillaLaunchConfig {
    pub fn new() -> Self {
        Self {
            runtime_major_version: None,
            launcher_name: None,
            launcher_version: None,
            client_id: None,
            resolution: None,
            quick_play: None,
        }
    }
}

#[async_trait]
impl Release for VanillaRelease {
    async fn install(&self) -> Result<()> {
        todo!()
    }

    async fn uninstall(&self) -> Result<()> {
        todo!()
    }

    async fn info(&self) -> ReleaseInfo {
        ReleaseInfo {
            name: self.version_id.clone(),
            game_versions: GameVersions::Single(self.version_id.clone()),
            description: self.description.clone(),
        }
    }
}

impl VanillaInstallStatus {
    pub fn is_persisted(&self) -> bool {
        self.metadata_persisted && self.asset_index_persisted
    }

    pub fn is_downloaded(&self) -> bool {
        self.is_persisted() && self.version_artifacts_ready && self.assets_ready
    }

    pub fn is_ready(&self) -> bool {
        self.is_downloaded() && self.natives_extracted
    }
}

impl ResolvedVanillaMetadata {
    pub async fn persist<L: Layout<Resource = Resource> + Clone, VL: Layout + Clone>(
        self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<ResolvedVanillaVersion<L, VL>> {
        instance.write_metadata(&self.metadata).await?;
        instance
            .parent
            .write_asset_index(
                self.metadata.asset_index.id.clone(),
                &self.asset_index_objects,
            )
            .await?;

        Ok(ResolvedVanillaVersion {
            endpoints: self.endpoints,
            version: instance.clone(),
            metadata: self.metadata,
            asset_index_objects: self.asset_index_objects,
        })
    }
}

impl<L: Layout<Resource = Resource>, VL: Layout> ResolvedVanillaVersion<L, VL> {
    pub fn load(endpoints: VanillaEndpoints, version: Storage<VL, Storage<L>>) -> Result<Self> {
        let metadata = version.metadata()?;
        let asset_index_objects = version
            .parent
            .asset_index_objects(metadata.asset_index.id.as_str())?;

        Ok(Self {
            endpoints,
            version,
            metadata,
            asset_index_objects,
        })
    }

    pub fn into_prepared(self) -> Result<PreparedVanillaVersion<L, VL>> {
        let status = self.status()?;
        if !status.is_ready() {
            let version_name = self.version.name().context("get version name failed")?;
            bail!("local version '{version_name}' is not prepared: {status:?}");
        }

        Ok(PreparedVanillaVersion {
            install_status: status,
            resolved_version: self,
        })
    }

    pub fn planner<'a>(&'a self) -> VanillaInstallPlanner<'a, L, VL> {
        VanillaInstallPlanner {
            version: &self.version,
            metadata: &self.metadata,
            asset_index_objects: &self.asset_index_objects,
            endpoints: self.endpoints.clone(),
            rule_context: MojangRuleContext::current(),
        }
    }

    pub fn required_java_major_version(&self) -> usize {
        self.metadata.java_version.major_version
    }

    pub fn status(&self) -> Result<VanillaInstallStatus> {
        let rule_context = MojangRuleContext::current();

        Ok(VanillaInstallStatus {
            metadata_persisted: self.version.metadata_path()?.exists(),
            asset_index_persisted: self
                .version
                .parent
                .asset_index_path(self.metadata.asset_index.id.as_str())?
                .exists(),
            version_artifacts_ready: self.version_artifacts_ready(&rule_context)?,
            assets_ready: self.assets_ready()?,
            natives_extracted: self.version.natives_are_extracted(),
        })
    }

    pub async fn prepare(
        self,
        downloader: &ElementalDownloader,
    ) -> Result<PreparedVanillaVersion<L, VL>> {
        let status = self.status()?;
        if !status.is_downloaded() {
            downloader.execute_planner(&self.planner()).await?;
        }

        let status = self.status()?;
        if !status.natives_extracted {
            self.version.extract_natives()?;
        }

        self.into_prepared()
    }

    fn version_artifacts_ready(&self, rule_context: &MojangRuleContext) -> Result<bool> {
        if !self.version.jar_path()?.exists() {
            return Ok(false);
        }

        for library in &self.metadata.libraries {
            if !library.is_allowed(rule_context) {
                continue;
            }

            if !self
                .version
                .parent
                .library_path(library.downloads.artifact.path.as_str())?
                .exists()
            {
                return Ok(false);
            }

            if let Some(artifact) = library.classifiers_native_artifact(rule_context.platform()) {
                if !self
                    .version
                    .parent
                    .library_path(artifact.path.as_str())?
                    .exists()
                {
                    return Ok(false);
                }
            }
        }

        if let Some(logging) = &self.metadata.logging {
            if !self
                .version
                .parent
                .logging_config_path(logging.client.file.id.as_str())?
                .exists()
            {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn assets_ready(&self) -> Result<bool> {
        for object in self.asset_index_objects.objects.values() {
            if !self
                .version
                .parent
                .asset_object_path(object.hash.as_str())?
                .exists()
            {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl<L: Layout<Resource = Resource>, VL: Layout> PreparedVanillaVersion<L, VL> {
    pub fn required_java_major_version(&self) -> usize {
        self.resolved_version.required_java_major_version()
    }
}

impl<'a, L: Layout<Resource = Resource>, VL: Layout> VanillaInstallPlanner<'a, L, VL> {
    fn version_name(&self) -> Result<String> {
        self.version.name().context("get version name failed")
    }

    fn plan_version_artifacts(&self) -> Result<DownloadPlan> {
        let mut tasks = Vec::new();
        tasks.push(DownloadTask::new(
            self.endpoints
                .rewrite(self.metadata.downloads.client.url.as_str())?,
            self.version.jar_path()?,
            Some(self.metadata.downloads.client.size as u64),
            Some(self.metadata.downloads.client.sha1.clone()),
        ));

        for library in &self.metadata.libraries {
            tasks.extend(self.plan_library_tasks(library)?);
        }

        if let Some(logging) = &self.metadata.logging {
            tasks.push(DownloadTask::new(
                self.endpoints.rewrite(logging.client.file.url.as_str())?,
                self.version
                    .parent
                    .logging_config_path(logging.client.file.id.as_str())?,
                Some(logging.client.file.size as u64),
                Some(logging.client.file.sha1.clone()),
            ));
        }

        Ok(DownloadPlan::named(self.version_name()?, tasks))
    }

    fn plan_library_tasks(&self, library: &PistonMetaLibraries) -> Result<Vec<DownloadTask>> {
        if !library.is_allowed(&self.rule_context) {
            return Ok(Vec::new());
        }

        let mut tasks = Vec::new();
        tasks.push(self.plan_library_artifact_task(&library.downloads.artifact)?);

        if let Some(artifact) = library.classifiers_native_artifact(self.rule_context.platform()) {
            tasks.push(self.plan_library_artifact_task(artifact)?);
        }

        Ok(tasks)
    }

    fn plan_library_artifact_task(
        &self,
        artifact: &PistonMetaLibrariesDownloadsArtifact,
    ) -> Result<DownloadTask> {
        Ok(DownloadTask::new(
            self.endpoints.rewrite(artifact.url.as_str())?,
            self.version.parent.library_path(artifact.path.as_str())?,
            Some(artifact.size as u64),
            Some(artifact.sha1.clone()),
        ))
    }

    fn plan_assets(&self) -> Result<DownloadPlan> {
        let version_name = self.version_name()?;
        let tasks = self
            .asset_index_objects
            .objects
            .values()
            .map(|object| {
                Ok(DownloadTask::new(
                    self.endpoints.object_url(object.hash.as_str())?,
                    self.version
                        .parent
                        .asset_object_path(object.hash.as_str())?,
                    Some(object.size as u64),
                    Some(object.hash.clone()),
                ))
            })
            .collect::<Result<Vec<DownloadTask>>>()?;

        Ok(DownloadPlan::named(format!("{version_name}-assets"), tasks))
    }
}

impl<'a, L: Layout<Resource = Resource>, VL: Layout> DownloadPlanner
    for VanillaInstallPlanner<'a, L, VL>
{
    fn plan(&self) -> Result<Vec<DownloadPlan>> {
        let mut plans = Vec::new();

        let version_plan = self.plan_version_artifacts()?;
        if !version_plan.tasks.is_empty() {
            plans.push(version_plan);
        }

        let assets_plan = self.plan_assets()?;
        if !assets_plan.tasks.is_empty() {
            plans.push(assets_plan);
        }

        Ok(plans)
    }
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

    pub async fn prepare<L: Layout<Resource = Resource> + Clone, VL: Layout + Clone>(
        &self,
        instance: &Storage<VL, Storage<L>>,
        version_id: String,
    ) -> Result<PreparedVanillaVersion<L, VL>> {
        let resolved = self.resolve_or_load(instance, version_id).await?;
        resolved.prepare(self.downloader()).await
    }

    pub fn load_prepared<L: Layout<Resource = Resource> + Clone, VL: Layout + Clone>(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedVanillaVersion<L, VL>> {
        ResolvedVanillaVersion::load(self.source.endpoints().clone(), instance.clone())?
            .into_prepared()
    }

    pub async fn launch<A, L: Layout<Resource = Resource> + Clone, VL: Layout + Clone>(
        &self,
        prepared_version: PreparedVanillaVersion<L, VL>,
        config: &VanillaLaunchConfig,
        authorizer: A,
    ) -> Result<LaunchedVanillaVersion<L, VL>>
    where
        A: Authorizer,
    {
        let (runtime, command) = self
            .build_launch_command(authorizer, &prepared_version, config)
            .await?;
        let child = process::spawn_command(command)?;

        Ok(LaunchedVanillaVersion {
            prepared_version,
            runtime,
            child,
        })
    }

    pub async fn build_launch_command<
        A,
        L: Layout<Resource = Resource> + Clone,
        VL: Layout + Clone,
    >(
        &self,
        authorizer: A,
        prepared_version: &PreparedVanillaVersion<L, VL>,
        config: &VanillaLaunchConfig,
    ) -> Result<(Distribution, LaunchCommand)>
    where
        A: Authorizer,
    {
        let runtime = self
            .runtime_for_prepared_version(prepared_version, config.runtime_major_version)
            .await?;
        let command = self
            .build_launch_builder(authorizer, runtime.clone(), prepared_version, config)?
            .build_command()
            .await?;

        Ok((runtime, command))
    }

    async fn runtime_for_prepared_version<L: Layout<Resource = Resource>, VL: Layout>(
        &self,
        prepared_version: &PreparedVanillaVersion<L, VL>,
        runtime_major_version: Option<usize>,
    ) -> Result<Distribution> {
        let required_major_version =
            runtime_major_version.unwrap_or_else(|| prepared_version.required_java_major_version());

        Distribution::find_cached_by_java_major(required_major_version)
            .await
            .with_context(|| {
                format!(
                    "can't find a local Java runtime with major version {}",
                    required_major_version
                )
            })
    }

    fn build_launch_builder<A, L: Layout<Resource = Resource> + Clone, VL: Layout + Clone>(
        &self,
        authorizer: A,
        runtime: Distribution,
        prepared_version: &PreparedVanillaVersion<L, VL>,
        config: &VanillaLaunchConfig,
    ) -> Result<MojangLaunchBuilder<A, L, VL>>
    where
        A: Authorizer,
    {
        let mut builder = MojangLaunchBuilder::new(
            authorizer,
            runtime,
            prepared_version.resolved_version.version.clone(),
        );

        if let Some(client_id) = &config.client_id {
            builder = builder.set_client_id(client_id.clone());
        }

        if let Some(resolution) = &config.resolution {
            builder = builder.set_resolution(resolution.width.clone(), resolution.height.clone());
        }

        if let (Some(name), Some(version)) = (&config.launcher_name, &config.launcher_version) {
            builder = builder.set_launcher(name.clone(), version.clone());
        }

        if let Some(quick_play) = &config.quick_play {
            builder = builder.set_quick_play(
                quick_play.path.clone(),
                quick_play.multiplayer.clone(),
                quick_play.singleplayer.clone(),
                quick_play.realms.clone(),
            );
        }

        Ok(builder)
    }

    async fn resolve_or_load<L: Layout<Resource = Resource> + Clone, VL: Layout + Clone>(
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

    async fn resolve_version<L: Layout<Resource = Resource> + Clone, VL: Layout + Clone>(
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

        Ok(ResolvedVanillaMetadata {
            endpoints: self.source.endpoints().clone(),
            metadata,
            asset_index_objects,
        })
    }
}

impl VanillaCatalog {
    pub fn new(source: VanillaSource) -> Self {
        Self { source }
    }

    pub fn with_defaults() -> Self {
        Self::new(VanillaSource::default())
    }
}

#[async_trait]
impl Catalog for VanillaCatalog {
    type Release = VanillaRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>> {
        let mut releases = HashMap::new();
        let manifest = self.source.launch_meta().await?;

        for version in manifest.versions {
            releases
                .entry(GameVersions::Single(version.id.clone()))
                .or_insert(Vec::new())
                .push(VanillaRelease {
                    version_id: version.id,
                    description: Some(version.release_type),
                });
        }

        Ok(releases)
    }
}

#[async_trait]
impl<L: Layout<Resource = Resource>, VL: Layout> Driver<L, VL> for VanillaDriver {
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
        if has_loader_marker(&metadata) {
            return Ok(None);
        }

        Ok(Some(InstalledDriver {
            driver: <Self as Driver<L, VL>>::descriptor(self),
            driver_version: None,
            game_version: Some(metadata.id.clone()),
            description: Some(metadata.release_type.clone()),
        }))
    }
}

fn has_loader_marker(metadata: &PistonMetaData) -> bool {
    metadata.libraries.iter().any(|library| {
        let name = library.name.as_str();
        name.starts_with("net.minecraftforge:forge:")
            || name.starts_with("net.neoforged:forge:")
            || name.starts_with("net.neoforged:neoforge:")
            || name.starts_with("net.fabricmc:fabric-loader:")
    })
}
