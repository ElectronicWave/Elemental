use elemental_core::storage::layout::Layoutable;
use elemental_infra::SafeRelativePath;

use crate::{
    error::Result,
    families::version_json::{
        PistonMetaLibrariesExt, VersionJsonInstanceLayout, VersionJsonInstanceResource,
        VersionJsonRemoteResolver, VersionJsonRootLayout, VersionJsonRuleContext,
        storage::library_artifacts_present,
    },
};

use super::ResolvedVersionJsonInstance;

impl<R, L, VL> ResolvedVersionJsonInstance<R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub(super) fn version_artifacts_ready(
        &self,
        rule_context: &VersionJsonRuleContext,
    ) -> Result<bool> {
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
        Ok(true)
    }

    fn libraries_ready(&self, rule_context: &VersionJsonRuleContext) -> Result<bool> {
        let mut artifact_paths = Vec::new();
        for library in self
            .metadata
            .libraries
            .iter()
            .filter(|library| library.is_allowed(rule_context))
        {
            for artifact in library.version_artifacts(rule_context.platform())? {
                artifact_paths.push(SafeRelativePath::parse(artifact.path.as_str())?);
            }
        }
        library_artifacts_present(&self.version.parent, artifact_paths)
    }
}
