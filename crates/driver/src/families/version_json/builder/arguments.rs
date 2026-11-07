use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use elemental_core::storage::layout::Layoutable;
use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::{Error, Result},
    families::version_json::{
        VersionJsonRootResource,
        classpath::{classpath_separator, join_classpath},
        extensions::PistonMetaLibrariesExt,
    },
};

use super::{
    VersionJsonLaunchBuilder,
    paths::{LaunchPaths, resolve_absolute_path},
};

pub(super) fn build_jvm_arguments<A, L, VL>(
    builder: &VersionJsonLaunchBuilder<A, L, VL>,
    metadata: &PistonMetaData,
    raw_jvm_arguments: Vec<String>,
) -> Result<Vec<String>>
where
    A: elemental_core::auth::authorizer::Authorizer,
    L: crate::families::version_json::VersionJsonRootLayout,
    VL: crate::families::version_json::VersionJsonInstanceLayout,
{
    let mut jvm = Vec::new();

    // Legacy metadata may already be normalized into `arguments.game`, while still needing
    // the historical JVM bootstrap arguments such as `-cp` and `-Djava.library.path`.
    if raw_jvm_arguments.is_empty() {
        jvm.extend(builder.inner.apply(legacy_jvm_arguments())?);
    }
    if let Some(logging_arguments) = logging_arguments(builder, metadata)? {
        jvm.extend(logging_arguments);
    }

    jvm.extend(builder.inner.apply(raw_jvm_arguments)?);
    jvm.extend(builder.inner.apply(builder.extra_jvm_arguments.clone())?);
    Ok(jvm)
}

pub(super) fn build_classpath<A, L, VL>(
    builder: &VersionJsonLaunchBuilder<A, L, VL>,
    metadata: &PistonMetaData,
    rule_context: &crate::families::version_json::VersionJsonRuleContext,
    paths: &LaunchPaths,
    module_path_entries: &HashSet<String>,
) -> Result<String>
where
    A: elemental_core::auth::authorizer::Authorizer,
    L: crate::families::version_json::VersionJsonRootLayout,
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

pub(super) fn collect_module_path_entries<A, L, VL>(
    builder: &VersionJsonLaunchBuilder<A, L, VL>,
    jvm_arguments: &[String],
) -> Result<HashSet<String>>
where
    A: elemental_core::auth::authorizer::Authorizer,
    L: crate::families::version_json::VersionJsonRootLayout,
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

fn logging_arguments<A, L, VL>(
    builder: &VersionJsonLaunchBuilder<A, L, VL>,
    metadata: &PistonMetaData,
) -> Result<Option<Vec<String>>>
where
    A: elemental_core::auth::authorizer::Authorizer,
    L: crate::families::version_json::VersionJsonRootLayout,
    VL: crate::families::version_json::VersionJsonInstanceLayout,
{
    let Some(client) = metadata
        .logging
        .as_ref()
        .and_then(|logging| logging.client.as_ref())
    else {
        return Ok(None);
    };

    let log_config_path = resolve_root_resource(
        builder,
        VersionJsonRootResource::AssetLogConfigs(Some(SafeRelativePath::parse(
            client.file.id.as_str(),
        )?)),
    )?;
    if !log_config_path.exists() {
        return Err(Error::LoggingConfigMissing {
            path: log_config_path,
        });
    }

    Ok(Some(builder.inner.apply_with(
        vec![client.argument.clone()],
        &HashMap::from([(
            "path".to_owned(),
            log_config_path.to_string_lossy().to_string(),
        )]),
    )?))
}

fn resolve_module_path_entries<A, L, VL>(
    builder: &VersionJsonLaunchBuilder<A, L, VL>,
    raw_value: &str,
) -> Result<Vec<String>>
where
    A: elemental_core::auth::authorizer::Authorizer,
    L: crate::families::version_json::VersionJsonRootLayout,
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

fn legacy_jvm_arguments() -> Vec<String> {
    vec![
        "-Djava.library.path=${natives_directory}".to_owned(),
        "-Dminecraft.launcher.brand=${launcher_name}".to_owned(),
        "-Dminecraft.launcher.version=${launcher_version}".to_owned(),
        "-cp".to_owned(),
        "${classpath}".to_owned(),
    ]
}

fn resolve_root_resource<A, L, VL>(
    builder: &VersionJsonLaunchBuilder<A, L, VL>,
    resource: VersionJsonRootResource,
) -> Result<PathBuf>
where
    A: elemental_core::auth::authorizer::Authorizer,
    L: crate::families::version_json::VersionJsonRootLayout,
    VL: crate::families::version_json::VersionJsonInstanceLayout,
{
    resolve_absolute_path(builder.version.parent.try_get_resource(resource)?)
}

fn normalize_path_string(path: &str) -> String {
    PathBuf::from(path)
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .to_string()
}
