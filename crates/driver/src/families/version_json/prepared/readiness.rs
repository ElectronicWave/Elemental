use elemental_core::storage::layout::Layoutable;
use elemental_infra::SafeRelativePath;

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonInstanceResource, VersionJsonRemoteResolver,
        VersionJsonRootLayout, VersionJsonRootResource, VersionJsonRuleContext,
        VersionJsonVersionStorageExt,
    },
};

use super::{ResolvedVersionJsonInstance, VersionJsonInstallStatus};

mod artifacts;

impl<R, L, VL> ResolvedVersionJsonInstance<R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub async fn status(&self) -> Result<VersionJsonInstallStatus> {
        let rule_context = VersionJsonRuleContext::current();
        Ok(VersionJsonInstallStatus {
            metadata_persisted: self
                .version
                .try_get_resource(VersionJsonInstanceResource::Metadata)?
                .exists(),
            asset_index_persisted: self
                .version
                .parent
                .try_get_resource(VersionJsonRootResource::AssetIndexes(Some(
                    SafeRelativePath::parse(self.metadata.asset_index.id.as_str())?,
                )))?
                .exists(),
            version_artifacts_ready: self.version_artifacts_ready(&rule_context)?,
            assets_ready: self.assets_ready()?,
            natives_extracted: self.version.natives_are_extracted().await,
        })
    }

    fn assets_ready(&self) -> Result<bool> {
        for object in self.asset_index_objects.objects.values() {
            if !self
                .version
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
}
