use elemental_infra::downloader::DownloadSession;

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonRootLayout, VersionJsonVersionStorageExt,
        remote::VersionJsonRemoteResolver,
    },
};

use super::super::{PreparedVersionJsonInstance, ResolvedVersionJsonInstance};

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
        if !self.status().await?.is_downloaded() {
            self.submit_install_batches(session).await?;
            session.success().await?;
        }
        self.finish_preparation().await
    }

    pub async fn submit_install_batches(&self, session: &DownloadSession) -> Result<()> {
        for batch in self.planner().download_batches()? {
            session.submit(batch).await?;
        }
        Ok(())
    }

    pub async fn finish_preparation(self) -> Result<PreparedVersionJsonInstance<L, VL>> {
        if !self.status().await?.natives_extracted {
            self.version.extract_natives().await?;
        }
        self.into_prepared().await
    }
}
