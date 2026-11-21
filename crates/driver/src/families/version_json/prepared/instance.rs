use std::collections::HashSet;

use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    error::{Error, Result},
    families::version_json::{
        VersionJsonAssetIndexStorageExt, VersionJsonInstanceLayout, VersionJsonMetadataStorageExt,
        VersionJsonRootLayout, VersionJsonRuleContext, classpath::resolve_classpath,
        remote::VersionJsonRemoteResolver,
    },
};

use super::VersionJsonInstallStatus;

mod metadata;
mod prepare;

pub(crate) use metadata::persist_version_metadata;
pub(crate) use prepare::{
    finish_instance_preparation, prepare_instance_with_download_status,
    submit_instance_install_batches,
};

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
    pub(crate) remote_resolver: R,
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
        let (metadata, asset_index_objects) = load_version_metadata(&version)?;
        Ok(Self {
            remote_resolver,
            version,
            metadata,
            asset_index_objects,
        })
    }

    pub async fn into_prepared(self) -> Result<PreparedVersionJsonInstance<L, VL>> {
        let status = self.status().await?;
        self.into_prepared_with_status(status).await
    }

    async fn into_prepared_with_status(
        self,
        status: VersionJsonInstallStatus,
    ) -> Result<PreparedVersionJsonInstance<L, VL>> {
        into_prepared_instance(self.version, self.metadata, status)
    }

    pub fn required_java_major_version(&self) -> usize {
        self.metadata.java_version.major_version
    }
}

pub(crate) fn load_version_metadata<L, VL>(
    version: &elemental_core::storage::Storage<VL, elemental_core::storage::Storage<L>>,
) -> Result<(PistonMetaData, PistonMetaAssetIndexObjects)>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let metadata = version.metadata()?;
    let asset_index_objects = version
        .parent
        .asset_index_objects(metadata.asset_index.id.as_str())?;
    Ok((metadata, asset_index_objects))
}

pub(crate) fn into_prepared_instance<L, VL>(
    version: elemental_core::storage::Storage<VL, elemental_core::storage::Storage<L>>,
    metadata: PistonMetaData,
    status: VersionJsonInstallStatus,
) -> Result<PreparedVersionJsonInstance<L, VL>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    if !status.is_ready() {
        let version_name = version.name().ok_or_else(|| Error::MissingVersionName {
            path: version.path.clone(),
        })?;
        return Err(Error::VersionNotPrepared {
            version: version_name,
            status: format!("{status:?}"),
        });
    }
    Ok(PreparedVersionJsonInstance {
        install_status: status,
        version,
        metadata,
    })
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

    pub fn classpath(&self) -> Result<Vec<std::path::PathBuf>> {
        resolve_classpath(
            &self.version,
            &self.metadata,
            &VersionJsonRuleContext::current(),
            &HashSet::new(),
        )
    }
}
