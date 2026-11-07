mod load;
mod persist;

use std::path::{Path, PathBuf};

use elemental_infra::SafeRelativePath;
use elemental_schema::forge::ForgeInstallerProfile;

use crate::error::Result;

pub(crate) use load::read_embedded_version;

const INSTALL_PROFILE_ENTRY: &str = "install_profile.json";

#[derive(Debug, Clone)]
pub struct InstallerPersistedState {
    pub install_profile: ForgeInstallerProfile,
    pub embedded_version: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub(crate) struct InstallerStatePaths<'a> {
    instance_root: &'a Path,
    family: SafeRelativePath,
}

impl<'a> InstallerStatePaths<'a> {
    pub(crate) fn new(instance_root: &'a Path, family_name: &str) -> Result<Self> {
        Ok(Self {
            instance_root,
            family: SafeRelativePath::parse(family_name.to_owned())?,
        })
    }

    pub(crate) fn install_profile_path(&self) -> PathBuf {
        self.relative_path(PathBuf::from(INSTALL_PROFILE_ENTRY))
    }

    pub(crate) fn embedded_version_path(&self) -> PathBuf {
        self.relative_path(PathBuf::from("version.json"))
    }

    fn root(&self) -> PathBuf {
        self.instance_root
            .join(".elemental")
            .join(self.family.as_path())
    }

    fn relative_path(&self, relative_path: PathBuf) -> PathBuf {
        self.root().join(relative_path)
    }
}
