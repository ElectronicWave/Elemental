use std::{env::current_dir, path::PathBuf};

use elemental_core::storage::{Storage, layout::Layoutable};

use crate::{
    error::{Error, Result},
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonInstanceResource, VersionJsonRootLayout,
        VersionJsonRootResource,
    },
};

#[derive(Debug, Clone)]
pub(super) struct LaunchPaths {
    pub(super) version_root: PathBuf,
    pub(super) version_jar: PathBuf,
    pub(super) assets_root: PathBuf,
    pub(super) libraries_root: PathBuf,
    pub(super) natives_directory: PathBuf,
}

impl LaunchPaths {
    pub(super) fn resolve<L, VL>(version: &Storage<VL, Storage<L>>) -> Result<Self>
    where
        L: VersionJsonRootLayout,
        VL: VersionJsonInstanceLayout,
    {
        Ok(Self {
            version_root: resolve_absolute_path(version.path.clone())?,
            version_jar: resolve_absolute_path(
                version.try_get_resource(VersionJsonInstanceResource::Jar)?,
            )?,
            assets_root: resolve_absolute_path(
                version
                    .parent
                    .try_get_resource(VersionJsonRootResource::Assets)?,
            )?,
            libraries_root: resolve_absolute_path(
                version
                    .parent
                    .try_get_resource(VersionJsonRootResource::Libraries(None))?,
            )?,
            natives_directory: resolve_absolute_path(
                version.try_get_resource(VersionJsonInstanceResource::Natives)?,
            )?,
        })
    }

    pub(super) fn ensure_version_jar_exists(&self) -> Result<()> {
        if self.version_jar.exists() {
            return Ok(());
        }

        Err(Error::VersionJarMissing {
            path: self.version_jar.clone(),
        })
    }
}

pub(super) fn resolve_absolute_path(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }

    Ok(current_dir()?.join(path))
}
