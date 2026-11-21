use elemental_infra::downloader::DownloadSession;

use crate::{
    error::Result,
    families::version_json::{
        remote::VersionJsonRemoteResolver, storage::ensure_natives_extracted,
    },
};

use super::super::{
    PreparedVersionJsonInstance, ResolvedVersionJsonInstance, VersionJsonDownloadStatus,
};

impl ResolvedVersionJsonInstance {
    pub async fn prepare(
        self,
        remote_resolver: &dyn VersionJsonRemoteResolver,
        session: &DownloadSession,
    ) -> Result<PreparedVersionJsonInstance> {
        self.prepare_with_download_status(remote_resolver, session, None)
            .await
    }

    pub(crate) async fn prepare_with_download_status(
        self,
        remote_resolver: &dyn VersionJsonRemoteResolver,
        session: &DownloadSession,
        download_status: Option<VersionJsonDownloadStatus>,
    ) -> Result<PreparedVersionJsonInstance> {
        let download_status = match download_status {
            Some(status) if status.is_downloaded() => status,
            _ => {
                self.submit_install_batches(remote_resolver, session)
                    .await?;
                session.success().await?;
                self.download_status()?
            }
        };
        self.finish_preparation_with_download_status(download_status)
            .await
    }

    pub async fn submit_install_batches(
        &self,
        remote_resolver: &dyn VersionJsonRemoteResolver,
        session: &DownloadSession,
    ) -> Result<()> {
        for batch in self.planner(remote_resolver).download_batches()? {
            session.submit(batch).await?;
        }
        Ok(())
    }

    pub async fn finish_preparation(self) -> Result<PreparedVersionJsonInstance> {
        let download_status = self.download_status()?;
        self.finish_preparation_with_download_status(download_status)
            .await
    }

    pub(crate) async fn finish_preparation_with_download_status(
        self,
        download_status: VersionJsonDownloadStatus,
    ) -> Result<PreparedVersionJsonInstance> {
        ensure_natives_extracted(&self.version, &self.metadata).await?;
        self.into_prepared_with_status(download_status.with_natives_extracted(true))
    }
}
