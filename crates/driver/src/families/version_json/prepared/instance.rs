use elemental_infra::downloader::DownloadSession;
use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    error::{Error, Result},
    families::version_json::{
        VersionJsonAssetIndexStorageExt, VersionJsonInstanceLayout, VersionJsonRootLayout,
        VersionJsonVersionStorageExt, remote::VersionJsonRemoteResolver,
    },
};

use super::VersionJsonInstallStatus;

#[derive(Debug, Clone)]
pub struct ResolvedVersionJsonMetadata<R: VersionJsonRemoteResolver> {
    pub(super) remote_resolver: R,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone)]
pub struct ResolvedVersionJsonInstance<
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
> {
    pub(super) remote_resolver: R,
    pub version: elemental_core::storage::Storage<VL, elemental_core::storage::Storage<L>>,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone)]
pub struct PreparedVersionJsonInstance<L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> {
    pub version: elemental_core::storage::Storage<VL, elemental_core::storage::Storage<L>>,
    pub metadata: PistonMetaData,
    pub install_status: VersionJsonInstallStatus,
}

impl<R: VersionJsonRemoteResolver> ResolvedVersionJsonMetadata<R> {
    pub fn new(
        remote_resolver: R,
        metadata: PistonMetaData,
        asset_index_objects: PistonMetaAssetIndexObjects,
    ) -> Self {
        Self {
            remote_resolver,
            metadata,
            asset_index_objects,
        }
    }

    pub async fn persist<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        self,
        instance: &elemental_core::storage::Storage<VL, elemental_core::storage::Storage<L>>,
    ) -> Result<ResolvedVersionJsonInstance<R, L, VL>> {
        instance.write_metadata(&self.metadata).await?;
        instance
            .parent
            .write_asset_index(
                self.metadata.asset_index.id.clone(),
                &self.asset_index_objects,
            )
            .await?;
        Ok(ResolvedVersionJsonInstance {
            remote_resolver: self.remote_resolver,
            version: instance.clone(),
            metadata: self.metadata,
            asset_index_objects: self.asset_index_objects,
        })
    }
}

impl<R, L, VL> ResolvedVersionJsonInstance<R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn load(
        remote_resolver: R,
        version: elemental_core::storage::Storage<VL, elemental_core::storage::Storage<L>>,
    ) -> Result<Self> {
        let metadata = version.metadata()?;
        let asset_index_objects = version
            .parent
            .asset_index_objects(metadata.asset_index.id.as_str())?;
        Ok(Self {
            remote_resolver,
            version,
            metadata,
            asset_index_objects,
        })
    }

    pub async fn into_prepared(self) -> Result<PreparedVersionJsonInstance<L, VL>> {
        let status = self.status().await?;
        if !status.is_ready() {
            let version = self
                .version
                .name()
                .ok_or_else(|| Error::MissingVersionName {
                    path: self.version.path.clone(),
                })?;
            return Err(Error::VersionNotPrepared {
                version,
                status: format!("{status:?}"),
            });
        }
        Ok(PreparedVersionJsonInstance {
            install_status: status,
            version: self.version,
            metadata: self.metadata,
        })
    }

    pub fn required_java_major_version(&self) -> usize {
        self.metadata.java_version.major_version
    }

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

impl<L, VL> PreparedVersionJsonInstance<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn instance_root(&self) -> &std::path::Path {
        self.version.path.as_path()
    }

    pub fn required_java_major_version(&self) -> usize {
        self.metadata.java_version.major_version
    }
}
