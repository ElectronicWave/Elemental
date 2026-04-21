use anyhow::{Context, Result, bail};
use elemental_core::{runtime::distribution::Distribution, storage::Storage};
use elemental_infra::downloader::{
    core::ElementalDownloader,
    plan::DownloadPlanner,
    report::SessionExecutionReport,
    task::{DownloadPlan, DownloadTask},
};

use crate::families::version_json::{
    PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaLibraries,
    PistonMetaLibrariesDownloadsArtifact, PistonMetaLibrariesExt, VersionJsonGameStorageExt,
    VersionJsonInstanceLayout, VersionJsonRootLayout, VersionJsonRuleContext,
    VersionJsonVersionStorageExt, remote::VersionJsonRemoteResolver,
};

#[derive(Debug, Clone)]
pub struct ResolvedVersionJsonMetadata<R: VersionJsonRemoteResolver> {
    remote_resolver: R,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionJsonInstallStatus {
    pub metadata_persisted: bool,
    pub asset_index_persisted: bool,
    pub version_artifacts_ready: bool,
    pub assets_ready: bool,
    pub natives_extracted: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedVersionJsonInstance<
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
> {
    remote_resolver: R,
    pub version: Storage<VL, Storage<L>>,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone)]
pub struct PreparedVersionJsonInstance<
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
> {
    pub resolved_version: ResolvedVersionJsonInstance<R, L, VL>,
    pub install_status: VersionJsonInstallStatus,
}

pub struct LaunchedVersionJsonInstance<
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
> {
    pub prepared_version: PreparedVersionJsonInstance<R, L, VL>,
    pub runtime: Distribution,
    pub child: tokio::process::Child,
}

pub struct VersionJsonInstallPlanner<
    'a,
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
> {
    version: &'a Storage<VL, Storage<L>>,
    metadata: &'a PistonMetaData,
    asset_index_objects: &'a PistonMetaAssetIndexObjects,
    remote_resolver: R,
    rule_context: VersionJsonRuleContext,
}

impl VersionJsonInstallStatus {
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

impl<R: VersionJsonRemoteResolver> ResolvedVersionJsonMetadata<R> {
    pub fn new(
        remote_resolver: R,
        metadata: PistonMetaData,
        asset_index_objects: PistonMetaAssetIndexObjects,
    ) -> Self {
        Self {
            remote_resolver,
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
    ) -> Result<ResolvedVersionJsonInstance<R, L, VL>> {
        instance.write_metadata(&self.metadata).await?;
        instance
            .parent
            .write_asset_index(
                self.metadata.asset_index.id.clone(),
                &self.asset_index_objects,
            )
            .await?;

        Ok(ResolvedVersionJsonInstance {
            remote_resolver: self.remote_resolver,
            version: instance.clone(),
            metadata: self.metadata,
            asset_index_objects: self.asset_index_objects,
        })
    }
}

impl<R, L, VL> ResolvedVersionJsonInstance<R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn load(remote_resolver: R, version: Storage<VL, Storage<L>>) -> Result<Self> {
        let metadata = version.metadata()?;
        let asset_index_objects = version
            .parent
            .asset_index_objects(metadata.asset_index.id.as_str())?;

        Ok(Self {
            remote_resolver,
            version,
            metadata,
            asset_index_objects,
        })
    }

    pub async fn into_prepared(self) -> Result<PreparedVersionJsonInstance<R, L, VL>> {
        let status = self.status().await?;
        if !status.is_ready() {
            let version_name = self.version.name().context("get version name failed")?;
            bail!("local version '{version_name}' is not prepared: {status:?}");
        }

        Ok(PreparedVersionJsonInstance {
            install_status: status,
            resolved_version: self,
        })
    }

    pub fn planner<'a>(&'a self) -> VersionJsonInstallPlanner<'a, R, L, VL> {
        VersionJsonInstallPlanner {
            version: &self.version,
            metadata: &self.metadata,
            asset_index_objects: &self.asset_index_objects,
            remote_resolver: self.remote_resolver.clone(),
            rule_context: VersionJsonRuleContext::current(),
        }
    }

    pub fn required_java_major_version(&self) -> usize {
        self.metadata.java_version.major_version
    }

    pub async fn status(&self) -> Result<VersionJsonInstallStatus> {
        let rule_context = VersionJsonRuleContext::current();

        Ok(VersionJsonInstallStatus {
            metadata_persisted: self.version.metadata_path()?.exists(),
            asset_index_persisted: self
                .version
                .parent
                .asset_index_path(self.metadata.asset_index.id.as_str())?
                .exists(),
            version_artifacts_ready: self.version_artifacts_ready(&rule_context)?,
            assets_ready: self.assets_ready()?,
            natives_extracted: self.version.natives_are_extracted().await,
        })
    }

    pub async fn prepare(
        self,
        downloader: &ElementalDownloader,
    ) -> Result<PreparedVersionJsonInstance<R, L, VL>> {
        let status = self.status().await?;
        if !status.is_downloaded() {
            let reports = downloader.execute_planner(&self.planner()).await?;
            ensure_download_reports_succeeded(&reports)?;
        }

        let status = self.status().await?;
        if !status.natives_extracted {
            self.version.extract_natives().await?;
        }

        self.into_prepared().await
    }

    fn version_artifacts_ready(&self, rule_context: &VersionJsonRuleContext) -> Result<bool> {
        if !self.version.jar_path()?.exists() {
            return Ok(false);
        }

        for library in &self.metadata.libraries {
            if !library.is_allowed(rule_context) {
                continue;
            }

            if let Some(artifact) = &library.downloads.artifact {
                if !self
                    .version
                    .parent
                    .library_path(artifact.path.as_str())?
                    .exists()
                {
                    return Ok(false);
                }
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

fn ensure_download_reports_succeeded(reports: &[SessionExecutionReport]) -> Result<()> {
    let failures = reports
        .iter()
        .flat_map(|report| {
            report
                .failures
                .iter()
                .map(|failure| format!("{}: {}", failure.task_id, failure.error))
        })
        .collect::<Vec<String>>();

    if !failures.is_empty() {
        bail!("version artifact download failed:\n{}", failures.join("\n"));
    }

    let cancelled = reports
        .iter()
        .flat_map(|report| report.cancelled_task_ids.iter().map(ToString::to_string))
        .collect::<Vec<String>>();

    if !cancelled.is_empty() {
        bail!(
            "version artifact download was cancelled:\n{}",
            cancelled.join("\n")
        );
    }

    Ok(())
}

impl<R, L, VL> PreparedVersionJsonInstance<R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn required_java_major_version(&self) -> usize {
        self.resolved_version.required_java_major_version()
    }
}

impl<'a, R, L, VL> VersionJsonInstallPlanner<'a, R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    fn version_name(&self) -> Result<String> {
        self.version.name().context("get version name failed")
    }

    fn plan_version_artifacts(&self) -> Result<DownloadPlan> {
        let mut tasks = Vec::new();
        tasks.push(DownloadTask::new(
            self.remote_resolver
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
                self.remote_resolver
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
        if let Some(artifact) = &library.downloads.artifact {
            tasks.push(self.plan_library_artifact_task(artifact)?);
        }

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
            self.remote_resolver
                .rewrite_upstream(artifact.url.as_str())?,
            self.version.parent.library_path(artifact.path.as_str())?,
            artifact.size.map(|size| size as u64),
            artifact.sha1.clone(),
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
                    self.remote_resolver.object_url(object.hash.as_str())?,
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

impl<'a, R, L, VL> DownloadPlanner for VersionJsonInstallPlanner<'a, R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
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
