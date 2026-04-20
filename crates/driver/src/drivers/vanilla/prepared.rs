use anyhow::{Context, Result, bail};
use elemental_core::{runtime::distribution::Distribution, storage::Storage};
use elemental_infra::downloader::{
    core::ElementalDownloader,
    plan::DownloadPlanner,
    task::{DownloadPlan, DownloadTask},
};

use crate::drivers::version_json::{
    PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaLibraries,
    PistonMetaLibrariesDownloadsArtifact, PistonMetaLibrariesExt, VersionJsonGameStorageExt,
    VersionJsonInstanceLayout, VersionJsonRootLayout, VersionJsonRuleContext,
    VersionJsonVersionStorageExt,
};

use super::source::VanillaEndpoints;

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
pub struct ResolvedVanillaVersion<L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> {
    endpoints: VanillaEndpoints,
    pub version: Storage<VL, Storage<L>>,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone)]
pub struct PreparedVanillaVersion<L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> {
    pub resolved_version: ResolvedVanillaVersion<L, VL>,
    pub install_status: VanillaInstallStatus,
}

pub struct LaunchedVanillaVersion<L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> {
    pub prepared_version: PreparedVanillaVersion<L, VL>,
    pub runtime: Distribution,
    pub child: tokio::process::Child,
}

pub struct VanillaInstallPlanner<'a, L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> {
    version: &'a Storage<VL, Storage<L>>,
    metadata: &'a PistonMetaData,
    asset_index_objects: &'a PistonMetaAssetIndexObjects,
    endpoints: VanillaEndpoints,
    rule_context: VersionJsonRuleContext,
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
    pub fn new(
        endpoints: VanillaEndpoints,
        metadata: PistonMetaData,
        asset_index_objects: PistonMetaAssetIndexObjects,
    ) -> Self {
        Self {
            endpoints,
            metadata,
            asset_index_objects,
        }
    }

    pub async fn persist<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
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

impl<L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> ResolvedVanillaVersion<L, VL> {
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
            rule_context: VersionJsonRuleContext::current(),
        }
    }

    pub fn required_java_major_version(&self) -> usize {
        self.metadata.java_version.major_version
    }

    pub fn status(&self) -> Result<VanillaInstallStatus> {
        let rule_context = VersionJsonRuleContext::current();

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

    fn version_artifacts_ready(&self, rule_context: &VersionJsonRuleContext) -> Result<bool> {
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

impl<L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> PreparedVanillaVersion<L, VL> {
    pub fn required_java_major_version(&self) -> usize {
        self.resolved_version.required_java_major_version()
    }
}

impl<'a, L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> VanillaInstallPlanner<'a, L, VL> {
    fn version_name(&self) -> Result<String> {
        self.version.name().context("get version name failed")
    }

    fn plan_version_artifacts(&self) -> Result<DownloadPlan> {
        let mut tasks = Vec::new();
        tasks.push(DownloadTask::new(
            self.endpoints
                .rewrite_upstream(self.metadata.downloads.client.url.as_str())?,
            self.version.jar_path()?,
            Some(self.metadata.downloads.client.size as u64),
            Some(self.metadata.downloads.client.sha1.clone()),
        ));

        for library in &self.metadata.libraries {
            tasks.extend(self.plan_library_tasks(library)?);
        }

        if let Some(logging) = &self.metadata.logging {
            tasks.push(DownloadTask::new(
                self.endpoints
                    .rewrite_upstream(logging.client.file.url.as_str())?,
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
            self.endpoints.rewrite_upstream(artifact.url.as_str())?,
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

impl<'a, L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> DownloadPlanner
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
