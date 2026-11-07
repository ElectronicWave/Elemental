use elemental_core::storage::layout::Layoutable;
use elemental_infra::SafeRelativePath;
use elemental_infra::downloader::{DownloadBatch, DownloadTask};
use elemental_schema::mojang::piston::{
    PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaLibraries,
    PistonMetaLibrariesDownloadsArtifact,
};

use crate::{
    error::Result,
    families::version_json::{
        PistonMetaLibrariesExt, VersionJsonInstanceLayout, VersionJsonInstanceResource,
        VersionJsonRemoteResolver, VersionJsonRootLayout, VersionJsonRootResource,
        VersionJsonRuleContext,
    },
};

use super::ResolvedVersionJsonInstance;

pub struct VersionJsonInstallPlanner<
    'a,
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
> {
    pub(super) version:
        &'a elemental_core::storage::Storage<VL, elemental_core::storage::Storage<L>>,
    pub(super) metadata: &'a PistonMetaData,
    pub(super) asset_index_objects: &'a PistonMetaAssetIndexObjects,
    pub(super) remote_resolver: R,
    pub(super) rule_context: VersionJsonRuleContext,
}

impl<R, L, VL> ResolvedVersionJsonInstance<R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn planner(&self) -> VersionJsonInstallPlanner<'_, R, L, VL> {
        VersionJsonInstallPlanner {
            version: &self.version,
            metadata: &self.metadata,
            asset_index_objects: &self.asset_index_objects,
            remote_resolver: self.remote_resolver.clone(),
            rule_context: VersionJsonRuleContext::current(),
        }
    }
}

impl<'a, R, L, VL> VersionJsonInstallPlanner<'a, R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn download_batches(&self) -> Result<Vec<DownloadBatch>> {
        let mut batches = Vec::new();
        let version = self.batch_version_artifacts()?;
        if !version.is_empty() {
            batches.push(version);
        }
        let assets = self.batch_assets()?;
        if !assets.is_empty() {
            batches.push(assets);
        }
        Ok(batches)
    }

    fn batch_version_artifacts(&self) -> Result<DownloadBatch> {
        let mut tasks = vec![self.client_task()?];
        for library in &self.metadata.libraries {
            tasks.extend(self.plan_library_tasks(library)?);
        }
        if let Some(task) = self.logging_task()? {
            tasks.push(task);
        }
        Ok(DownloadBatch::new(tasks).group("version"))
    }

    fn client_task(&self) -> Result<DownloadTask> {
        let client = &self.metadata.downloads.client;
        Ok(DownloadTask::new(
            self.remote_resolver.rewrite_upstream(client.url.as_str())?,
            self.version
                .try_get_resource(VersionJsonInstanceResource::Jar)?,
        )
        .size(client.size as u64)
        .sha1(client.sha1.clone()))
    }

    fn logging_task(&self) -> Result<Option<DownloadTask>> {
        let Some(client) = self
            .metadata
            .logging
            .as_ref()
            .and_then(|logging| logging.client.as_ref())
        else {
            return Ok(None);
        };
        Ok(Some(
            DownloadTask::new(
                self.remote_resolver
                    .rewrite_upstream(client.file.url.as_str())?,
                self.version
                    .parent
                    .try_get_resource(VersionJsonRootResource::AssetLogConfigs(Some(
                        SafeRelativePath::parse(client.file.id.as_str())?,
                    )))?,
            )
            .size(client.file.size as u64)
            .sha1(client.file.sha1.clone()),
        ))
    }

    fn plan_library_tasks(&self, library: &PistonMetaLibraries) -> Result<Vec<DownloadTask>> {
        if !library.is_allowed(&self.rule_context) {
            return Ok(Vec::new());
        }
        library
            .version_artifacts(self.rule_context.platform())
            .into_iter()
            .flatten()
            .map(|artifact| self.plan_library_artifact_task(artifact))
            .collect()
    }

    fn plan_library_artifact_task(
        &self,
        artifact: &PistonMetaLibrariesDownloadsArtifact,
    ) -> Result<DownloadTask> {
        let mut task = DownloadTask::new(
            self.remote_resolver
                .rewrite_upstream(artifact.url.as_str())?,
            self.version
                .parent
                .try_get_resource(VersionJsonRootResource::Libraries(Some(
                    SafeRelativePath::parse(artifact.path.as_str())?,
                )))?,
        );
        if let Some(size) = artifact.size {
            task = task.size(size as u64);
        }
        if let Some(sha1) = &artifact.sha1 {
            task = task.sha1(sha1.clone());
        }
        Ok(task)
    }

    fn batch_assets(&self) -> Result<DownloadBatch> {
        let tasks = self
            .asset_index_objects
            .objects
            .values()
            .map(|object| {
                Ok(DownloadTask::new(
                    self.remote_resolver.object_url(object.hash.as_str())?,
                    self.version
                        .parent
                        .try_get_resource(VersionJsonRootResource::AssetObjects(Some(
                            SafeRelativePath::parse(object.hash.as_str())?,
                        )))?,
                )
                .size(object.size as u64)
                .sha1(object.hash.clone()))
            })
            .collect::<Result<Vec<DownloadTask>>>()?;
        Ok(DownloadBatch::new(tasks).group("assets"))
    }
}
