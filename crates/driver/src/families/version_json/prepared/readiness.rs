use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonInstanceResource, VersionJsonRemoteResolver,
        VersionJsonRootLayout, VersionJsonRootResource, VersionJsonRuleContext,
        storage::natives_are_extracted,
    },
};

use super::{ResolvedVersionJsonInstance, VersionJsonDownloadStatus, VersionJsonInstallStatus};

mod artifacts;

impl<R, L, VL> ResolvedVersionJsonInstance<R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub async fn download_status(&self) -> Result<VersionJsonDownloadStatus> {
        check_download_status(&self.version, &self.metadata, &self.asset_index_objects)
    }

    pub async fn status(&self) -> Result<VersionJsonInstallStatus> {
        check_install_status(&self.version, &self.metadata, &self.asset_index_objects).await
    }
}

pub(crate) fn check_download_status<L, VL>(
    version: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
    asset_index_objects: &PistonMetaAssetIndexObjects,
) -> Result<VersionJsonDownloadStatus>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let rule_context = VersionJsonRuleContext::current();
    Ok(VersionJsonDownloadStatus {
        metadata_persisted: version
            .try_get_resource(VersionJsonInstanceResource::Metadata)?
            .exists(),
        asset_index_persisted: version
            .parent
            .try_get_resource(VersionJsonRootResource::AssetIndexes(Some(
                SafeRelativePath::parse(metadata.asset_index.id.as_str())?,
            )))?
            .exists(),
        version_artifacts_ready: artifacts::version_artifacts_ready(
            version,
            metadata,
            &rule_context,
        )?,
        assets_ready: assets_ready(version, asset_index_objects)?,
    })
}

pub(crate) async fn check_install_status<L, VL>(
    version: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
    asset_index_objects: &PistonMetaAssetIndexObjects,
) -> Result<VersionJsonInstallStatus>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let download_status = check_download_status(version, metadata, asset_index_objects)?;
    let natives_extracted = natives_are_extracted(version, metadata).await?;
    Ok(download_status.with_natives_extracted(natives_extracted))
}

fn assets_ready<L, VL>(
    version: &Storage<VL, Storage<L>>,
    asset_index_objects: &PistonMetaAssetIndexObjects,
) -> Result<bool>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    for object in asset_index_objects.objects.values() {
        if !version
            .parent
            .try_get_resource(VersionJsonRootResource::AssetObjects(Some(
                SafeRelativePath::parse(object.hash.as_str())?,
            )))?
            .exists()
        {
            return Ok(false);
        }
    }
    Ok(true)
}
