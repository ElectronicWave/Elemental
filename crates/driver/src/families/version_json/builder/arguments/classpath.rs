use std::collections::HashSet;

use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonRootLayout, VersionJsonRootResource, classpath::join_classpath,
        extensions::PistonMetaLibrariesExt,
    },
};

use super::super::{VersionJsonLaunchBuilder, paths::LaunchPaths};
use super::common::{normalize_path_string, resolve_root_resource};

pub(crate) fn build_classpath<L, VL>(
    builder: &VersionJsonLaunchBuilder<L, VL>,
    metadata: &PistonMetaData,
    rule_context: &crate::families::version_json::VersionJsonRuleContext,
    paths: &LaunchPaths,
    module_path_entries: &HashSet<String>,
) -> Result<String>
where
    L: VersionJsonRootLayout,
    VL: crate::families::version_json::VersionJsonInstanceLayout,
{
    let mut classpath = metadata
        .libraries
        .iter()
        .map(|library| -> Result<Option<String>> {
            if !library.is_allowed(rule_context) {
                return Ok(None);
            }
            let Some(artifact) = library.downloads.artifact.as_ref() else {
                return Ok(None);
            };
            if artifact.path.contains("natives") {
                return Ok(None);
            }

            let path = resolve_root_resource(
                builder,
                VersionJsonRootResource::Libraries(Some(SafeRelativePath::parse(
                    artifact.path.as_str(),
                )?)),
            )?
            .to_string_lossy()
            .to_string();
            if module_path_entries.contains(&normalize_path_string(path.as_str())) {
                return Ok(None);
            }

            Ok(Some(path))
        })
        .collect::<Result<Vec<Option<String>>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<String>>();

    classpath.push(paths.version_jar.to_string_lossy().to_string());
    Ok(join_classpath(classpath))
}
