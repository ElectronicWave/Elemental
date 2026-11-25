use std::path::PathBuf;

use elemental_core::storage::layout::Layoutable;

use crate::{
    error::Result,
    families::version_json::{
        InstanceStorage, VersionJsonInstanceResource, VersionJsonRootResource,
        classpath::resolve_absolute_path,
    },
};

#[derive(Debug, Clone)]
pub(super) struct LaunchPaths {
    pub(super) version_root: PathBuf,
    pub(super) assets_root: PathBuf,
    pub(super) libraries_root: PathBuf,
    pub(super) natives_directory: PathBuf,
}

impl LaunchPaths {
    pub(super) fn resolve(version: &InstanceStorage) -> Result<Self> {
        Ok(Self {
            version_root: resolve_absolute_path(version.path.clone())?,
            assets_root: resolve_absolute_path(
                version.try_get_parent_resource(VersionJsonRootResource::Assets)?,
            )?,
            libraries_root: resolve_absolute_path(
                version.try_get_parent_resource(VersionJsonRootResource::Libraries(None))?,
            )?,
            natives_directory: resolve_absolute_path(
                version.try_get_resource(VersionJsonInstanceResource::Natives)?,
            )?,
        })
    }
}
