use std::collections::HashSet;

use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonRootLayout,
        classpath::{join_classpath, resolve_classpath},
    },
};

use super::super::VersionJsonLaunchBuilder;

pub(crate) fn build_classpath<L, VL>(
    builder: &VersionJsonLaunchBuilder<L, VL>,
    metadata: &PistonMetaData,
    rule_context: &crate::families::version_json::VersionJsonRuleContext,
    module_path_entries: &HashSet<String>,
) -> Result<String>
where
    L: VersionJsonRootLayout,
    VL: crate::families::version_json::VersionJsonInstanceLayout,
{
    let classpath = resolve_classpath(
        &builder.version,
        metadata,
        rule_context,
        module_path_entries,
    )?;
    Ok(join_classpath(
        classpath
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
    ))
}
