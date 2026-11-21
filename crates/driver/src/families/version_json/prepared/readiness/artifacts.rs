use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::Result,
    families::version_json::{
        PistonMetaLibrariesExt, VersionJsonInstanceLayout, VersionJsonInstanceResource,
        VersionJsonRootLayout, VersionJsonRuleContext, storage::library_artifacts_present,
    },
};

pub(super) fn version_artifacts_ready<L, VL>(
    version: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
    rule_context: &VersionJsonRuleContext,
) -> Result<bool>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    if !version
        .try_get_resource(VersionJsonInstanceResource::Jar)?
        .exists()
    {
        return Ok(false);
    }
    if !libraries_ready(version, metadata, rule_context)? {
        return Ok(false);
    }
    Ok(true)
}

fn libraries_ready<L, VL>(
    version: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
    rule_context: &VersionJsonRuleContext,
) -> Result<bool>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let mut artifact_paths = Vec::new();
    for library in metadata
        .libraries
        .iter()
        .filter(|library| library.is_allowed(rule_context))
    {
        for artifact in library.version_artifacts(rule_context.platform())? {
            artifact_paths.push(SafeRelativePath::parse(artifact.path.as_str())?);
        }
    }
    library_artifacts_present(&version.parent, artifact_paths)
}
