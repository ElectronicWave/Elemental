use std::path::PathBuf;

use elemental_core::storage::layout::Layoutable;

use crate::{
    error::Result,
    families::version_json::{VersionJsonRootLayout, VersionJsonRootResource},
};

use super::super::{VersionJsonLaunchBuilder, paths::resolve_absolute_path};

pub(super) fn legacy_jvm_arguments() -> Vec<String> {
    vec![
        "-Djava.library.path=${natives_directory}".to_owned(),
        "-Dminecraft.launcher.brand=${launcher_name}".to_owned(),
        "-Dminecraft.launcher.version=${launcher_version}".to_owned(),
        "-cp".to_owned(),
        "${classpath}".to_owned(),
    ]
}

pub(super) fn resolve_root_resource<A, L, VL>(
    builder: &VersionJsonLaunchBuilder<A, L, VL>,
    resource: VersionJsonRootResource,
) -> Result<PathBuf>
where
    A: elemental_core::auth::authorizer::Authorizer,
    L: VersionJsonRootLayout,
    VL: crate::families::version_json::VersionJsonInstanceLayout,
{
    resolve_absolute_path(builder.version.parent.try_get_resource(resource)?)
}

pub(super) fn normalize_path_string(path: &str) -> String {
    PathBuf::from(path)
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .to_string()
}
