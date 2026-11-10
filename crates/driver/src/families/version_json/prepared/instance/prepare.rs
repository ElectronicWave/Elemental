use elemental_infra::downloader::DownloadSession;

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonRootLayout, remote::VersionJsonRemoteResolver,
        storage::ensure_natives_extracted,
    },
};

use super::super::{
    PreparedVersionJsonInstance, ResolvedVersionJsonInstance, VersionJsonDownloadStatus,
};

impl<R, L, VL> ResolvedVersionJsonInstance<R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub async fn prepare(
        self,
        session: &DownloadSession,
    ) -> Result<PreparedVersionJsonInstance<L, VL>> {
        self.prepare_with_download_status(session, None).await
    }

    pub(crate) async fn prepare_with_download_status(
        self,
        session: &DownloadSession,
        download_status: Option<VersionJsonDownloadStatus>,
    ) -> Result<PreparedVersionJsonInstance<L, VL>> {
        let download_status = match download_status {
            Some(status) if status.is_downloaded() => status,
            _ => self.download_missing(session).await?,
        };
        self.finish_preparation_with_download_status(download_status)
            .await
    }

    async fn download_missing(
        &self,
        session: &DownloadSession,
    ) -> Result<VersionJsonDownloadStatus> {
        let status = self.download_status().await?;
        if status.is_downloaded() {
            return Ok(status);
        }
        self.submit_install_batches(session).await?;
        session.success().await?;
        self.download_status().await
    }

    pub async fn submit_install_batches(&self, session: &DownloadSession) -> Result<()> {
        for batch in self.planner().download_batches()? {
            session.submit(batch).await?;
        }
        Ok(())
    }

    pub async fn finish_preparation(self) -> Result<PreparedVersionJsonInstance<L, VL>> {
        let download_status = self.download_status().await?;
        self.finish_preparation_with_download_status(download_status)
            .await
    }

    async fn finish_preparation_with_download_status(
        self,
        download_status: VersionJsonDownloadStatus,
    ) -> Result<PreparedVersionJsonInstance<L, VL>> {
        ensure_natives_extracted(&self.version, &self.metadata).await?;
        self.into_prepared_with_status(download_status.with_natives_extracted(true))
            .await
    }
}
