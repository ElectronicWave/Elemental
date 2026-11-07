use elemental_core::storage::layout::Layoutable;
use elemental_infra::{SafeRelativePath, downloader::DownloadTask};

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonInstanceResource, VersionJsonRemoteResolver,
        VersionJsonRootLayout, VersionJsonRootResource,
    },
};

use super::VersionJsonInstallPlanner;

impl<R, L, VL> VersionJsonInstallPlanner<'_, R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub(super) fn batch_version_artifacts(
        &self,
    ) -> Result<elemental_infra::downloader::DownloadBatch> {
        let mut tasks = vec![self.client_task()?];
        for library in &self.metadata.libraries {
            tasks.extend(self.plan_library_tasks(library)?);
        }
        if let Some(task) = self.logging_task()? {
            tasks.push(task);
        }
        Ok(elemental_infra::downloader::DownloadBatch::new(tasks).group("version"))
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
}
