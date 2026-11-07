use std::path::Path;

use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    error::{Error, Result},
    families::installer::InstallerArchive,
};

use super::{InstallerPersistedState, InstallerStatePaths};

impl InstallerStatePaths<'_> {
    pub(crate) fn load(&self) -> Result<InstallerPersistedState> {
        let install_profile_path = self.install_profile_path();
        let install_profile = serde_json::from_reader(std::fs::File::open(&install_profile_path)?)
            .map_err(|source| Error::ReadPersistedInstallProfile {
                family_name: self.family.as_str().to_owned(),
                path: install_profile_path.clone(),
                source,
            })?;

        let embedded_version_path = self.embedded_version_path();
        let embedded_version = if embedded_version_path.exists() {
            Some(
                serde_json::from_reader(std::fs::File::open(&embedded_version_path)?).map_err(
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
) -> Result<Option<serde_json::Value>> {
    if let Some(version_info) = &install_profile.version_info {
        return Ok(Some(version_info.clone()));
    }

    let Some(json_path) = install_profile.json.as_deref() else {
        return Ok(None);
    };

    Ok(Some(
        archive
            .read_json::<serde_json::Value>(json_path)
            .map_err(|source| Error::ReadEmbeddedVersionJson {
                family_name: family_name.to_owned(),
                path: json_path.to_owned(),
                source: Box::new(source),
            })?,
    ))
}
