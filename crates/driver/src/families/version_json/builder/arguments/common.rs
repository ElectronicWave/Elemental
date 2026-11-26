use std::path::PathBuf;

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonRootResource, classpath::resolve_absolute_path, layout::InstanceStorage,
    },
};

pub(super) fn legacy_jvm_arguments() -> Vec<String> {
    vec![
        "-Djava.library.path=${natives_directory}".to_owned(),
        "-Dminecraft.launcher.brand=${launcher_name}".to_owned(),
        "-Dminecraft.launcher.version=${launcher_version}".to_owned(),
        "-cp".to_owned(),
        "${classpath}".to_owned(),
    ]
}

pub(super) fn resolve_root_resource(
    version: &InstanceStorage,
    resource: VersionJsonRootResource,
) -> Result<PathBuf> {
    resolve_absolute_path(version.try_get_parent_resource(resource)?)
}

pub(super) fn normalize_path_string(path: &str) -> String {
    PathBuf::from(path)
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .to_string()
}
