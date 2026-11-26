use elemental_core::storage::layout::Layoutable;
use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    error::Result,
    families::version_json::{
        InstanceStorage, VersionJsonInstanceResource, VersionJsonRootResource,
        VersionJsonRuleContext, storage::natives_are_extracted,
    },
};

use super::{VersionJsonDownloadStatus, VersionJsonInstallStatus};

mod artifacts;

pub(super) fn download_status(
    version: &InstanceStorage,
    metadata: &PistonMetaData,
    asset_index_objects: &PistonMetaAssetIndexObjects,
) -> Result<VersionJsonDownloadStatus> {
    let rule_context = VersionJsonRuleContext::current();
    Ok(VersionJsonDownloadStatus {
        metadata_persisted: version
            .try_get_resource(VersionJsonInstanceResource::Metadata)?
            .exists(),
        asset_index_persisted: version
            .try_get_parent_resource(VersionJsonRootResource::AssetIndexes(Some(
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

pub(super) async fn status(
    version: &InstanceStorage,
    metadata: &PistonMetaData,
    asset_index_objects: &PistonMetaAssetIndexObjects,
) -> Result<VersionJsonInstallStatus> {
    let download_status = download_status(version, metadata, asset_index_objects)?;
    let natives_extracted = natives_are_extracted(version, metadata).await?;
    Ok(download_status.with_natives_extracted(natives_extracted))
}

fn assets_ready(
    version: &InstanceStorage,
    asset_index_objects: &PistonMetaAssetIndexObjects,
) -> Result<bool> {
    for object in asset_index_objects.objects.values() {
        if !version
            .try_get_parent_resource(VersionJsonRootResource::AssetObjects(Some(
                SafeRelativePath::parse(object.hash.as_str())?,
            )))?
            .exists()
        {
            return Ok(false);
        }
    }
    Ok(true)
}
