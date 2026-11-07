use elemental_core::storage::layout::Layoutable;
use elemental_infra::SafeRelativePath;

use crate::{
    error::Result,
    families::version_json::storage::library_artifacts_present,
    families::version_json::{
        PistonMetaLibrariesExt, VersionJsonInstanceLayout, VersionJsonInstanceResource,
        VersionJsonRemoteResolver, VersionJsonRootLayout, VersionJsonRootResource,
        VersionJsonRuleContext, VersionJsonVersionStorageExt,
    },
};

use super::{ResolvedVersionJsonInstance, VersionJsonInstallStatus};

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

    fn version_artifacts_ready(&self, rule_context: &VersionJsonRuleContext) -> Result<bool> {
        if !self
            .version
            .try_get_resource(VersionJsonInstanceResource::Jar)?
            .exists()
        {
            return Ok(false);
        }
        if !self.libraries_ready(rule_context)? {
            return Ok(false);
        }
        self.logging_config_ready()
    }

    fn libraries_ready(&self, rule_context: &VersionJsonRuleContext) -> Result<bool> {
        let artifact_paths = self
            .metadata
            .libraries
            .iter()
            .filter(|library| library.is_allowed(rule_context))
            .flat_map(|library| library.version_artifacts(rule_context.platform()))
            .flatten()
            .map(|artifact| SafeRelativePath::parse(artifact.path.as_str()))
            .collect::<std::result::Result<Vec<SafeRelativePath>, _>>()?;
        library_artifacts_present(&self.version.parent, artifact_paths)
    }

    fn logging_config_ready(&self) -> Result<bool> {
        let Some(client) = self
            .metadata
            .logging
            .as_ref()
            .and_then(|logging| logging.client.as_ref())
        else {
            return Ok(true);
        };
        Ok(self
            .version
            .parent
            .try_get_resource(VersionJsonRootResource::AssetLogConfigs(Some(
                SafeRelativePath::parse(client.file.id.as_str())?,
            )))?
            .exists())
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
