use elemental_core::storage::Storage;
use elemental_infra::downloader::DownloadSession;
use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonRootLayout, VersionJsonRuleContext,
        prepared::{planner::DynInstallPlanner, readiness::check_download_status},
        remote::{DynRemoteResolver, VersionJsonRemoteResolver},
        storage::ensure_natives_extracted,
    },
};

use super::{
    super::{PreparedVersionJsonInstance, ResolvedVersionJsonInstance, VersionJsonDownloadStatus},
    into_prepared_instance,
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
        prepare_instance_with_download_status(
            &self.remote_resolver,
            self.version,
            self.metadata,
            self.asset_index_objects,
            session,
            download_status,
        )
        .await
    }

    pub async fn submit_install_batches(&self, session: &DownloadSession) -> Result<()> {
        submit_instance_install_batches(
            &self.remote_resolver,
            &self.version,
            &self.metadata,
            &self.asset_index_objects,
            session,
        )
        .await
    }

    pub async fn finish_preparation(self) -> Result<PreparedVersionJsonInstance<L, VL>> {
        let download_status =
            check_download_status(&self.version, &self.metadata, &self.asset_index_objects)?;
        finish_instance_preparation(self.version, self.metadata, download_status).await
    }
}

pub(crate) async fn submit_instance_install_batches<L, VL>(
    remote_resolver: &dyn DynRemoteResolver,
    version: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
    asset_index_objects: &PistonMetaAssetIndexObjects,
    session: &DownloadSession,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let rule_context = VersionJsonRuleContext::current();
    let batches = DynInstallPlanner {
        version,
        metadata,
        asset_index_objects,
        remote_resolver,
        rule_context: &rule_context,
    }
    .download_batches()?;
    for batch in batches {
        session.submit(batch).await?;
    }
    Ok(())
}

pub(crate) async fn finish_instance_preparation<L, VL>(
    version: Storage<VL, Storage<L>>,
    metadata: PistonMetaData,
    download_status: VersionJsonDownloadStatus,
) -> Result<PreparedVersionJsonInstance<L, VL>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    ensure_natives_extracted(&version, &metadata).await?;
    into_prepared_instance(
        version,
        metadata,
        download_status.with_natives_extracted(true),
    )
}

pub(crate) async fn prepare_instance_with_download_status<L, VL>(
    remote_resolver: &dyn DynRemoteResolver,
    version: Storage<VL, Storage<L>>,
    metadata: PistonMetaData,
    asset_index_objects: PistonMetaAssetIndexObjects,
    session: &DownloadSession,
    download_status: Option<VersionJsonDownloadStatus>,
) -> Result<PreparedVersionJsonInstance<L, VL>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let download_status = match download_status {
        Some(status) if status.is_downloaded() => status,
        _ => {
            submit_instance_install_batches(
                remote_resolver,
                &version,
                &metadata,
                &asset_index_objects,
                session,
            )
            .await?;
            session.success().await?;
            check_download_status(&version, &metadata, &asset_index_objects)?
        }
    };
    finish_instance_preparation(version, metadata, download_status).await
}
