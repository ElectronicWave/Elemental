use std::path::{Path, PathBuf};

use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::SafeRelativePath;
use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    error::{Error, Result},
    families::{
        installer::{InstallerArchive, InstallerArtifact},
        version_json::{
            VersionJsonInstanceLayout, VersionJsonInstanceResource, VersionJsonRootLayout,
            VersionJsonRootResource,
        },
    },
};

use super::InstallerProcessorContext;

impl<'a> InstallerProcessorContext<'a> {
    pub(in crate::families::installer::processor) fn new<L, VL>(
        instance: &Storage<VL, Storage<L>>,
        installer_artifact: &'a InstallerArtifact,
        install_profile: &'a ForgeInstallerProfile,
        archive: InstallerArchive<PathBuf>,
        family_name: &'a str,
    ) -> Result<Self>
    where
        L: VersionJsonRootLayout,
        VL: VersionJsonInstanceLayout,
    {
        Ok(Self {
            installer_artifact,
            install_profile,
            archive,
            root_directory: absolute_path(&instance.parent.path)?,
            libraries_directory: absolute_path(
                &instance
                    .parent
                    .try_get_resource(VersionJsonRootResource::Libraries(None))?,
            )?,
            state_directory: absolute_path(&instance.try_get_resource(
                VersionJsonInstanceResource::Elemental(Some(SafeRelativePath::parse(format!(
                    "{family_name}/installer"
                ))?)),
            )?)?,
            minecraft_jar_path: absolute_path(
                &instance.try_get_resource(VersionJsonInstanceResource::Jar)?,
            )?,
            family_name,
        })
    }
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let current_directory = std::env::current_dir().map_err(|source| Error::CurrentDirectory {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(current_directory.join(path))
}
