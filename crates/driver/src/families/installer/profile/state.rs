mod load;
mod persist;

use std::path::PathBuf;

use elemental_core::storage::layout::Layoutable;
use elemental_infra::SafeRelativePath;
use elemental_schema::forge::ForgeInstallerProfile;
use serde_json::value::RawValue;

use crate::{
    error::Result,
    families::version_json::{InstanceStorage, VersionJsonInstanceResource},
};

pub(crate) use load::read_embedded_version;

const INSTALL_PROFILE_ENTRY: &str = "install_profile.json";

#[derive(Debug, Clone)]
pub struct InstallerPersistedState {
    pub install_profile: ForgeInstallerProfile,
    pub embedded_version: Option<Box<RawValue>>,
}

#[derive(Debug, Clone)]
pub(crate) struct InstallerStatePaths {
    root: PathBuf,
    family: SafeRelativePath,
}

impl InstallerStatePaths {
    pub(crate) fn new(instance: &InstanceStorage, family_name: &str) -> Result<Self> {
        let family = SafeRelativePath::parse(family_name.to_owned())?;
        let root = instance
            .try_get_resource(VersionJsonInstanceResource::Elemental(Some(family.clone())))?;
        Ok(Self { root, family })
    }

    pub(crate) fn install_profile_path(&self) -> PathBuf {
        self.root.join(INSTALL_PROFILE_ENTRY)
    }

    pub(crate) fn embedded_version_path(&self) -> PathBuf {
        self.root.join("version.json")
    }
}
