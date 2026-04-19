use anyhow::{Context, Result};
use elemental_infra::downloader::{
    plan::DownloadPlanner,
    task::{DownloadPlan, DownloadTask},
};

use crate::{
    mojang::{
        MojangBaseUrl, MojangRuleContext, PistonMetaAssetIndexObjects, PistonMetaData,
        PistonMetaLibraries, PistonMetaLibrariesDownloadsArtifact, PistonMetaLibrariesExt,
    },
    services::mojang::MojangService,
    storage::{game::GameStorage, layout::Layout, version::VersionStorage},
};

#[derive(Debug)]
pub struct ResolvedVanillaVersion<L: Layout, VL: Layout> {
    baseurl: MojangBaseUrl,
    pub version: VersionStorage<L, VL>,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

impl<L: Layout, VL: Layout> ResolvedVanillaVersion<L, VL> {
    pub fn planner<'a>(&'a self) -> VanillaInstallPlanner<'a, L, VL> {
        VanillaInstallPlanner {
            version: &self.version,
            metadata: &self.metadata,
            asset_index_objects: &self.asset_index_objects,
            baseurl: self.baseurl.clone(),
            rule_context: MojangRuleContext::current(),
        }
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
            self.metadata
                .downloads
                .client
                .url
                .replace("piston-data.mojang.com", &self.baseurl.pistondata),
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
            artifact
                .url
                .replace("libraries.minecraft.net", &self.baseurl.libraries),
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

impl MojangService {
    pub async fn resolve_vanilla_version<L: Layout + Clone, VL: Layout>(
        &self,
        storage: &GameStorage<L>,
        version_id: impl Into<String>,
        version_name: impl Into<String>,
        version_layout: VL,
    ) -> Result<ResolvedVanillaVersion<L, VL>> {
        let version_id = version_id.into();
        let version_name = version_name.into();
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
        let version = storage.ensure_version(version_name, version_layout).await?;

        version.write_metadata(&metadata).await?;
        storage
            .write_asset_index(metadata.asset_index.id.as_str(), &asset_index_objects)
            .await?;

        Ok(ResolvedVanillaVersion {
            baseurl: self.baseurl.clone(),
            version,
            metadata,
            asset_index_objects,
        })
    }
}
