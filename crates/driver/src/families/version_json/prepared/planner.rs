use elemental_infra::downloader::DownloadBatch;
use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonRemoteResolver, VersionJsonRootLayout,
        VersionJsonRuleContext,
    },
};

use super::ResolvedVersionJsonInstance;

mod assets;
mod libraries;
mod version;

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
}
