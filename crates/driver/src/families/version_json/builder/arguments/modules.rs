use std::collections::HashSet;

use crate::{
    error::{Error, Result},
    families::version_json::{VersionJsonRootLayout, classpath::classpath_separator},
};

use super::super::VersionJsonLaunchBuilder;
use super::common::normalize_path_string;

pub(crate) fn collect_module_path_entries<A, L, VL>(
    builder: &VersionJsonLaunchBuilder<A, L, VL>,
    jvm_arguments: &[String],
) -> Result<HashSet<String>>
where
    A: elemental_core::auth::authorizer::Authorizer,
    L: VersionJsonRootLayout,
    VL: crate::families::version_json::VersionJsonInstanceLayout,
{
    let mut entries = HashSet::new();
    let mut index = 0usize;

    while index < jvm_arguments.len() {
        let argument = &jvm_arguments[index];
        let inline_value = argument
            .strip_prefix("--module-path=")
            .or_else(|| argument.strip_prefix("-p="));

        if let Some(raw_value) = inline_value {
            entries.extend(resolve_module_path_entries(builder, raw_value)?);
            index += 1;
        } else if argument == "-p" || argument == "--module-path" {
            let raw_value = jvm_arguments
                .get(index + 1)
                .ok_or(Error::ModulePathValueMissing)?;
            entries.extend(resolve_module_path_entries(builder, raw_value)?);
            index += 2;
        } else {
            index += 1;
        }
    }

    Ok(entries)
}

fn resolve_module_path_entries<A, L, VL>(
    builder: &VersionJsonLaunchBuilder<A, L, VL>,
    raw_value: &str,
) -> Result<Vec<String>>
where
    A: elemental_core::auth::authorizer::Authorizer,
    L: VersionJsonRootLayout,
    VL: crate::families::version_json::VersionJsonInstanceLayout,
{
    let resolved = builder
        .inner
        .apply(vec![raw_value.to_owned()])?
        .into_iter()
        .next()
        .ok_or(Error::ResolveModulePath)?;

    Ok(resolved
        .split(classpath_separator())
        .filter(|value| !value.is_empty())
        .map(normalize_path_string)
        .collect())
}
