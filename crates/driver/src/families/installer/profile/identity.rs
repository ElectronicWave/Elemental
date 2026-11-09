use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    error::{Error, Result},
    loader_version::LoaderVersionId,
    maven::MavenCoordinate,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstallerProfileIdentity {
    game_version: String,
    loader_version: LoaderVersionId,
}

impl InstallerProfileIdentity {
    pub(crate) fn from_profile(
        install_profile: &ForgeInstallerProfile,
        family_name: &str,
        loader_name: &str,
    ) -> Result<Self> {
        let game_version = install_profile
            .minecraft
            .clone()
            .or_else(|| {
                install_profile
                    .install
                    .as_ref()
                    .and_then(|legacy| legacy.minecraft.clone())
            })
            .ok_or_else(|| Error::InstallerProfileMinecraftVersionMissing {
                family_name: family_name.to_owned(),
            })?;

        let raw_loader_version = install_profile
            .path
            .clone()
            .or_else(|| {
                install_profile
                    .install
                    .as_ref()
                    .and_then(|legacy| legacy.path.clone())
            })
            .and_then(|coordinate| {
                MavenCoordinate::parse(coordinate.as_str())
                    .map(|parsed| parsed.version().to_owned())
            })
            .or_else(|| install_profile.version.clone())
            .ok_or_else(|| Error::InstallerProfileLoaderVersionMissing {
                family_name: family_name.to_owned(),
                loader_name: loader_name.to_owned(),
            })?;

        Ok(Self {
            game_version,
            loader_version: LoaderVersionId::from(raw_loader_version),
        })
    }

    fn validate_requested(
        &self,
        expected_game_version: &str,
        expected_loader_version: &LoaderVersionId,
        family_name: &str,
    ) -> Result<()> {
        if self.game_version != expected_game_version {
            return Err(Error::InstallerGameVersionMismatch {
                family_name: family_name.to_owned(),
                actual: self.game_version.clone(),
                expected: expected_game_version.to_owned(),
            });
        }

        if &self.loader_version != expected_loader_version {
            return Err(Error::InstallerLoaderVersionMismatch {
                family_name: family_name.to_owned(),
                actual: self.loader_version.to_string(),
                expected: expected_loader_version.to_string(),
            });
        }

        Ok(())
    }
}

pub fn profile_game_and_raw_loader_version(
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
    loader_name: &str,
) -> Result<(String, LoaderVersionId)> {
    let identity =
        InstallerProfileIdentity::from_profile(install_profile, family_name, loader_name)?;
    Ok((identity.game_version, identity.loader_version))
}

pub fn validate_installer_profile_identity(
    expected_game_version: &str,
    expected_loader_version: &LoaderVersionId,
    actual_game_version: &str,
    actual_loader_version: &LoaderVersionId,
    family_name: &str,
) -> Result<()> {
    InstallerProfileIdentity {
        game_version: actual_game_version.to_owned(),
        loader_version: actual_loader_version.clone(),
    }
    .validate_requested(expected_game_version, expected_loader_version, family_name)
}
