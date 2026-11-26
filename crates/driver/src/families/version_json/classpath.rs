use std::{
    collections::HashSet,
    env::current_dir,
    path::{Path, PathBuf},
};

use elemental_core::storage::layout::Layoutable;
use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::Result,
    families::version_json::{
        InstanceStorage, VersionJsonInstanceResource, VersionJsonRootResource,
        VersionJsonRuleContext, extensions::PistonMetaLibrariesExt,
    },
};

pub(crate) fn resolve_absolute_path(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }

    Ok(current_dir()?.join(path))
}

pub fn classpath_separator() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

pub fn join_classpath(entries: Vec<String>) -> String {
    entries.join(classpath_separator())
}

pub fn resolve_classpath(
    version: &InstanceStorage,
    metadata: &PistonMetaData,
    rule_context: &VersionJsonRuleContext,
    module_path_entries: &HashSet<String>,
) -> Result<Vec<PathBuf>> {
    let mut classpath = Vec::new();

    for library in metadata
        .libraries
        .iter()
        .filter(|library| library.is_allowed(rule_context))
    {
        let downloads = library.downloads()?;
        let Some(artifact) = downloads.artifact.as_ref() else {
            continue;
        };

        let relative_path = SafeRelativePath::parse(artifact.path.as_str())?;
        let path = resolve_absolute_path(
            version
                .try_get_parent_resource(VersionJsonRootResource::Libraries(Some(relative_path)))?,
        )?;
        if module_path_entries.contains(&normalize_path_string(&path)) {
            continue;
        }

        classpath.push(path);
    }

    classpath.push(resolve_absolute_path(
        version.try_get_resource(VersionJsonInstanceResource::Jar)?,
    )?);
    Ok(classpath)
}

fn normalize_path_string(path: &Path) -> String {
    path.components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .to_string()
}
