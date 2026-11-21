use std::path::Path;

use elemental_schema::forge::ForgeInstallerProfile;
use serde_json::value::RawValue;

use crate::{
    error::{Error, Result},
    families::installer::InstallerArchive,
};

use super::{InstallerPersistedState, InstallerStatePaths};

impl InstallerStatePaths<'_> {
    pub(crate) fn load_install_profile(&self) -> Result<ForgeInstallerProfile> {
        let install_profile_path = self.install_profile_path();
        let install_profile_bytes = std::fs::read(&install_profile_path)?;
        serde_json::from_slice(&install_profile_bytes).map_err(|source| {
            Error::ReadPersistedInstallProfile {
                family_name: self.family.as_str().to_owned(),
                path: install_profile_path,
                source,
            }
        })
    }

    pub(crate) fn load(&self) -> Result<InstallerPersistedState> {
        let install_profile = self.load_install_profile()?;

        let embedded_version_path = self.embedded_version_path();
        let embedded_version = if embedded_version_path.exists() {
            let embedded_version_bytes = std::fs::read(&embedded_version_path)?;
            Some(
                serde_json::from_slice::<Box<RawValue>>(&embedded_version_bytes).map_err(
                    |source| Error::ReadPersistedEmbeddedVersion {
                        family_name: self.family.as_str().to_owned(),
                        path: embedded_version_path.clone(),
                        source,
                    },
                )?,
            )
        } else {
            None
        };

        Ok(InstallerPersistedState {
            install_profile,
            embedded_version,
        })
    }
}

pub(crate) fn read_embedded_version<P: AsRef<Path>>(
    archive: &InstallerArchive<P>,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
) -> Result<Option<Box<RawValue>>> {
    if let Some(version_info) = &install_profile.version_info {
        return Ok(Some(version_info.clone()));
    }

    let Some(json_path) = install_profile.json.as_deref() else {
        return Ok(None);
    };

    let bytes = archive
        .read_bytes(json_path)
        .map_err(|source| Error::ReadEmbeddedVersionJson {
            family_name: family_name.to_owned(),
            path: json_path.to_owned(),
            source: Box::new(source),
        })?;
    let raw = serde_json::from_slice::<Box<RawValue>>(&bytes).map_err(|source| {
        Error::ReadEmbeddedVersionJson {
            family_name: family_name.to_owned(),
            path: json_path.to_owned(),
            source: Box::new(crate::error::Error::from(source)),
        }
    })?;
    Ok(Some(raw))
}
