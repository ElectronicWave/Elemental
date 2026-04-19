use anyhow::{Context, Result};
use elemental_infra::downloader::{
    core::ElementalDownloader,
    plan::DownloadPlanner,
    task::{DownloadPlan, DownloadTask},
};

use crate::{
    mojang::{
        MojangBaseUrl, MojangRuleContext, PistonMetaAssetIndexObjects, PistonMetaData,
        PistonMetaLibraries, PistonMetaLibrariesDownloadsArtifact, PistonMetaLibrariesExt,
    },
    services::mojang::MojangClient,
    storage::{game::GameStorage, layout::Layout, version::VersionStorage},
};

#[derive(Debug, Clone)]
pub struct ResolvedVanillaMetadata {
    baseurl: MojangBaseUrl,
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
pub struct ResolvedVanillaVersion<L: Layout, VL: Layout> {
    baseurl: MojangBaseUrl,
    pub version: VersionStorage<L, VL>,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone)]
pub struct ReadyVanillaVersion<L: Layout, VL: Layout> {
    pub resolved_version: ResolvedVanillaVersion<L, VL>,
    pub install_status: VanillaInstallStatus,
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
    pub async fn persist<L: Layout + Clone, VL: Layout>(
        self,
        storage: &GameStorage<L>,
        version_name: impl Into<String>,
        version_layout: VL,
    ) -> Result<ResolvedVanillaVersion<L, VL>> {
        let version = storage.ensure_version(version_name, version_layout).await?;
        version.write_metadata(&self.metadata).await?;
        storage
            .write_asset_index(
                self.metadata.asset_index.id.as_str(),
                &self.asset_index_objects,
            )
            .await?;

        Ok(ResolvedVanillaVersion {
            baseurl: self.baseurl,
            version,
            metadata: self.metadata,
            asset_index_objects: self.asset_index_objects,
        })
    }
}

impl<L: Layout, VL: Layout> ResolvedVanillaVersion<L, VL> {
    pub fn load(baseurl: MojangBaseUrl, version: VersionStorage<L, VL>) -> Result<Self> {
        let metadata = version.metadata()?;
        let asset_index_objects = version
            .global
            .asset_index_objects(metadata.asset_index.id.as_str())?;

        Ok(Self {
            baseurl,
            version,
            metadata,
            asset_index_objects,
        })
    }

    pub fn planner<'a>(&'a self) -> VanillaInstallPlanner<'a, L, VL> {
        VanillaInstallPlanner {
            version: &self.version,
            metadata: &self.metadata,
            asset_index_objects: &self.asset_index_objects,
            baseurl: self.baseurl.clone(),
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
                .global
                .asset_index_path(self.metadata.asset_index.id.as_str())?
                .exists(),
            version_artifacts_ready: self.version_artifacts_ready(&rule_context)?,
            assets_ready: self.assets_ready()?,
            natives_extracted: self.version.natives_are_extracted(),
        })
    }

    pub async fn ready(
        self,
        downloader: &ElementalDownloader,
    ) -> Result<ReadyVanillaVersion<L, VL>> {
        let status = self.status()?;
        if !status.is_downloaded() {
            downloader.execute_planner(&self.planner()).await?;
        }

        let status = self.status()?;
        if !status.natives_extracted {
            self.version.extract_natives()?;
        }

        Ok(ReadyVanillaVersion {
            install_status: self.status()?,
            resolved_version: self,
        })
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
                .global
                .library_path(library.downloads.artifact.path.as_str())?
                .exists()
            {
                return Ok(false);
            }

            if let Some(artifact) = library.classifiers_native_artifact(rule_context.platform()) {
                if !self
                    .version
                    .global
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
                .global
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
                .global
                .asset_object_path(object.hash.as_str())?
                .exists()
            {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl<L: Layout, VL: Layout> ReadyVanillaVersion<L, VL> {
    pub fn required_java_major_version(&self) -> usize {
        self.resolved_version.required_java_major_version()
    }
}

pub struct VanillaInstallPlanner<'a, L: Layout, VL: Layout> {
    version: &'a VersionStorage<L, VL>,
    metadata: &'a PistonMetaData,
    asset_index_objects: &'a PistonMetaAssetIndexObjects,
    baseurl: MojangBaseUrl,
    rule_context: MojangRuleContext,
}

impl<'a, L: Layout, VL: Layout> VanillaInstallPlanner<'a, L, VL> {
    fn version_name(&self) -> Result<String> {
        self.version.name().context("get version name failed")
    }

    fn plan_version_artifacts(&self) -> Result<DownloadPlan> {
        let mut tasks = Vec::new();
        tasks.push(DownloadTask::new(
            self.baseurl
                .rewrite_pistondata_url(self.metadata.downloads.client.url.clone()),
            self.version.jar_path()?,
            Some(self.metadata.downloads.client.size as u64),
            Some(self.metadata.downloads.client.sha1.clone()),
        ));

        for library in &self.metadata.libraries {
            tasks.extend(self.plan_library_tasks(library)?);
        }

        if let Some(logging) = &self.metadata.logging {
            tasks.push(DownloadTask::new(
                logging
                    .client
                    .file
                    .url
                    .replace("piston-data.mojang.com", &self.baseurl.pistondata)
                    .replace("launchermeta.mojang.com", &self.baseurl.launchermeta),
                self.version
                    .global
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
            self.baseurl.rewrite_library_url(artifact.url.clone()),
            self.version.global.library_path(artifact.path.as_str())?,
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
                    self.baseurl.get_object_url(object.hash.clone()),
                    self.version
                        .global
                        .asset_object_path(object.hash.as_str())?,
                    Some(object.size as u64),
                    Some(object.hash.clone()),
                ))
            })
            .collect::<Result<Vec<DownloadTask>>>()?;

        Ok(DownloadPlan::named(format!("{version_name}-assets"), tasks))
    }
}

impl<'a, L: Layout, VL: Layout> DownloadPlanner for VanillaInstallPlanner<'a, L, VL> {
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

impl MojangClient {
    pub async fn resolve_vanilla_metadata(
        &self,
        version_id: impl Into<String>,
    ) -> Result<ResolvedVanillaMetadata> {
        let version_id = version_id.into();
        let launchmeta = self.launchmeta().await?;
        let metadata_url = launchmeta
            .versions
            .iter()
            .find(|version| version.id == version_id)
            .context(format!("Can't find version named `{version_id}`"))?
            .url
            .clone();
        let metadata = self.pistonmeta(metadata_url).await?;
        let asset_index_objects = self
            .pistonmeta_assetindex_objects(metadata.asset_index.url.clone())
            .await?;

        Ok(ResolvedVanillaMetadata {
            baseurl: self.baseurl.clone(),
            metadata,
            asset_index_objects,
        })
    }

    pub async fn resolve_vanilla_version<L: Layout + Clone, VL: Layout>(
        &self,
        storage: &GameStorage<L>,
        version_id: impl Into<String>,
        version_name: impl Into<String>,
        version_layout: VL,
    ) -> Result<ResolvedVanillaVersion<L, VL>> {
        self.resolve_vanilla_metadata(version_id)
            .await?
            .persist(storage, version_name, version_layout)
            .await
    }
}
