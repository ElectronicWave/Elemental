mod common;
mod logging;
mod modules;

use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::Result,
    families::version_json::{layout::InstanceStorage, variables::LauncherVariables},
};

pub(crate) use modules::collect_module_path_entries;

pub(super) fn build_jvm_arguments(
    version: &InstanceStorage,
    variables: &LauncherVariables,
    extra_jvm_arguments: &[String],
    metadata: &PistonMetaData,
    raw_jvm_arguments: Vec<String>,
) -> Result<Vec<String>> {
    let mut jvm = Vec::new();

    // Legacy metadata may already be normalized into `arguments.game`, while still needing
    // the historical JVM bootstrap arguments such as `-cp` and `-Djava.library.path`.
    if raw_jvm_arguments.is_empty() {
        jvm.extend(variables.apply(&common::legacy_jvm_arguments()));
    }
    if let Some(logging_arguments) = logging::logging_arguments(version, variables, metadata)? {
        jvm.extend(logging_arguments);
    }

    jvm.extend(variables.apply(&raw_jvm_arguments));
    jvm.extend(variables.apply(extra_jvm_arguments));
    Ok(jvm)
}
