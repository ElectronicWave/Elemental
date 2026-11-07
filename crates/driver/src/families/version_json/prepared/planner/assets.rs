use elemental_core::storage::layout::Layoutable;
use elemental_infra::{SafeRelativePath, downloader::DownloadTask};

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonRemoteResolver, VersionJsonRootLayout,
        VersionJsonRootResource,
    },
};

use super::VersionJsonInstallPlanner;

impl<R, L, VL> VersionJsonInstallPlanner<'_, R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub(super) fn batch_assets(&self) -> Result<elemental_infra::downloader::DownloadBatch> {
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
        Ok(elemental_infra::downloader::DownloadBatch::new(tasks).group("assets"))
    }
}
