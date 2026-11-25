mod client;
mod endpoints;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_infra::SafeRelativePath;
use elemental_schema::{forge::ForgeInstallerProfile, mojang::piston::PistonMetaLibraries};

use crate::{
    catalog::LoaderRelease,
    descriptors::DriverDescriptor,
    error::{Error, Result},
    families::installer::profile_game_and_raw_loader_version,
    inspect::ProfiledDriverIdentity,
    loader_version::LoaderVersionId,
    maven::MavenCoordinate,
};

#[cfg(feature = "maven-xml")]
pub use client::InstallerMavenCatalog;
pub use client::InstallerMavenSource;
pub use endpoints::MavenEndpoints;

#[derive(Debug, Clone)]
pub struct InstallerMavenArtifactSpec {
    pub coordinate: String,
    pub download_url: String,
    pub relative_path: SafeRelativePath,
}

#[derive(Debug, Clone, Copy)]
pub struct MavenInstallerSpec {
    pub identity: ProfiledDriverIdentity,
    pub family_name: &'static str,
    pub source_name: &'static str,
    pub loader_version_prefix: &'static str,
    pub canonical_origin: &'static str,
    pub group: &'static str,
    pub artifact: &'static str,
    pub fixed_game_version: Option<&'static str>,
    pub format_version: fn(&MinecraftVersionId, &LoaderVersionId) -> String,
    pub parse_release:
        fn(&MavenInstallerSpec, String) -> Option<(MinecraftVersionId, LoaderRelease)>,
    pub merge_libraries:
        fn(Vec<PistonMetaLibraries>, Vec<PistonMetaLibraries>) -> Vec<PistonMetaLibraries>,
}

impl MavenInstallerSpec {
    pub fn descriptor(&self) -> DriverDescriptor {
        self.identity.descriptor()
    }

    pub fn parse_release(&self, version: String) -> Option<(MinecraftVersionId, LoaderRelease)> {
        (self.parse_release)(self, version)
    }

    pub fn version(
        &self,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> String {
        (self.format_version)(game_version, loader_version)
    }

    pub fn coordinate(&self, version: &str) -> Result<MavenCoordinate> {
        let notation = format!("{}:{}:{version}", self.group, self.artifact);
        MavenCoordinate::parse(&notation).ok_or_else(|| Error::InvalidNotation {
            label: self.source_name.to_owned(),
            notation,
        })
    }

    pub fn metadata_path(&self) -> Result<String> {
        let directory = MavenCoordinate::artifact_directory_path_for(self.group, self.artifact)
            .ok_or_else(|| Error::InvalidNotation {
                label: self.source_name.to_owned(),
                notation: format!("{}:{}", self.group, self.artifact),
            })?;
        Ok(format!("/{directory}/maven-metadata.xml"))
    }

    pub fn installer_coordinate(&self, version: &str) -> Result<MavenCoordinate> {
        let coordinate = self.coordinate(version)?;
        coordinate
            .with_classifier("installer")
            .ok_or_else(|| Error::InvalidNotation {
                label: self.source_name.to_owned(),
                notation: coordinate.to_string(),
            })
    }

    pub fn profile_identity(
        &self,
        install_profile: &ForgeInstallerProfile,
    ) -> Result<(String, LoaderVersionId)> {
        let (game_version, raw_version) = profile_game_and_raw_loader_version(
            install_profile,
            self.family_name,
            self.family_name,
        )?;
        let loader_version = normalize_profile_loader_version(
            raw_version.as_str(),
            game_version.as_str(),
            self.loader_version_prefix,
        );
        Ok((game_version, LoaderVersionId::from(loader_version)))
    }

    pub fn merge_libraries(
        &self,
        base_libraries: Vec<PistonMetaLibraries>,
        embedded_libraries: Vec<PistonMetaLibraries>,
    ) -> Vec<PistonMetaLibraries> {
        (self.merge_libraries)(base_libraries, embedded_libraries)
    }
}

fn normalize_profile_loader_version<'a>(
    raw_version: &'a str,
    game_version: &str,
    loader_prefix: &str,
) -> &'a str {
    let without_game = raw_version
        .strip_prefix(game_version)
        .and_then(|version| version.strip_prefix('-'))
        .unwrap_or(raw_version);

    without_game
        .split_once(loader_prefix)
        .map_or(without_game, |(_, loader_version)| loader_version)
}
