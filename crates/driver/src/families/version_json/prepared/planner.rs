use elemental_infra::{
    SafeRelativePath,
    downloader::{DownloadBatch, DownloadTask},
};
use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    error::Result,
    families::version_json::{
        InstanceStorage, VersionJsonRemoteResolver, VersionJsonRootResource, VersionJsonRuleContext,
    },
};

mod libraries;
mod version;

pub struct VersionJsonInstallPlanner<'a> {
    pub(super) version: &'a InstanceStorage,
    pub(super) metadata: &'a PistonMetaData,
    pub(super) asset_index_objects: &'a PistonMetaAssetIndexObjects,
    pub(super) remote_resolver: &'a dyn VersionJsonRemoteResolver,
    pub(super) rule_context: VersionJsonRuleContext,
}

impl<'a> VersionJsonInstallPlanner<'a> {
    pub(super) fn new(
        version: &'a InstanceStorage,
        metadata: &'a PistonMetaData,
        asset_index_objects: &'a PistonMetaAssetIndexObjects,
        remote_resolver: &'a dyn VersionJsonRemoteResolver,
    ) -> Self {
        Self {
            version,
            metadata,
            asset_index_objects,
            remote_resolver,
            rule_context: VersionJsonRuleContext::current(),
        }
    }
}

impl VersionJsonInstallPlanner<'_> {
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

    fn batch_assets(&self) -> Result<DownloadBatch> {
        let tasks = self
            .asset_index_objects
            .objects
            .values()
            .map(|object| {
                Ok(DownloadTask::new(
                    self.remote_resolver.object_url(object.hash.as_str())?,
                    self.version
                        .try_get_parent_resource(VersionJsonRootResource::AssetObjects(Some(
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
