use std::fmt::Debug;

use elemental_core::{minecraft::MinecraftVersionId, storage::Storage};
use elemental_schema::{forge::ForgeInstallerProfile, mojang::piston::PistonMetaLibraries};

use crate::{
    driver::DriverDescriptor,
    error::Result,
    families::{
        installer::{
            InstallerArtifact, InstallerInstallStatus, merge_libraries_prefer_embedded,
            normalize_library_urls, profile_game_and_raw_loader_version,
        },
        version_json::VersionJsonRootLayout,
    },
    inspect::ProfiledDriverIdentity,
    loader_version::LoaderVersionId,
};

mod prepared;
mod resolved;
mod resolver;

pub use resolved::ResolvedInstallerFamilyVersion;
pub use resolver::{
    InstallerFamilyRemoteResolver, ResolvedInstallerFamilyLaunchVersion,
    ResolvedInstallerFamilyMetadata,
};

pub type InstallerFamilyInstallStatus = InstallerInstallStatus;

pub trait InstallerArtifactEndpoints: Clone + Debug + Send + Sync + 'static {
    fn artifact_url(&self, artifact_path: &str) -> Result<String>;
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String>;
}

pub trait InstallerArtifactSource: Clone + Debug {
    type Endpoints: InstallerArtifactEndpoints;

    fn endpoints(&self) -> &Self::Endpoints;

    fn installer_artifact<L>(
        &self,
        game_storage: &Storage<L>,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerArtifact>
    where
        L: VersionJsonRootLayout;
}

pub trait InstallerFamily: Clone + Copy + Debug + Send + Sync + 'static {
    type Source: InstallerArtifactSource<Endpoints = Self::Endpoints>;
    type Endpoints: InstallerArtifactEndpoints;

    const IDENTITY: ProfiledDriverIdentity;
    const FAMILY_NAME: &'static str;
    const LOADER_VERSION_PREFIX: &'static str;

    fn descriptor() -> DriverDescriptor {
        Self::IDENTITY.descriptor()
    }

    fn source_endpoints(source: &Self::Source) -> &Self::Endpoints {
        source.endpoints()
    }

    fn installer_artifact<L>(
        source: &Self::Source,
        game_storage: &Storage<L>,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerArtifact>
    where
        L: VersionJsonRootLayout,
    {
        source.installer_artifact(game_storage, game_version, loader_version)
    }

    fn profile_identity(
        install_profile: &ForgeInstallerProfile,
    ) -> Result<(String, LoaderVersionId)> {
        let (game_version, raw_version) = profile_game_and_raw_loader_version(
            install_profile,
            Self::FAMILY_NAME,
            Self::FAMILY_NAME,
        )?;
        let loader_version = normalize_profile_loader_version(
            raw_version.as_str(),
            game_version.as_str(),
            Self::LOADER_VERSION_PREFIX,
        );
        Ok((game_version, LoaderVersionId::from(loader_version)))
    }

    fn normalize_libraries(
        endpoints: &Self::Endpoints,
        libraries: Vec<PistonMetaLibraries>,
    ) -> Result<Vec<PistonMetaLibraries>> {
        normalize_library_urls(libraries, |artifact_path| {
            endpoints.artifact_url(artifact_path)
        })
    }

    fn merge_libraries(
        base_libraries: Vec<PistonMetaLibraries>,
        embedded_libraries: Vec<PistonMetaLibraries>,
    ) -> Vec<PistonMetaLibraries> {
        merge_libraries_prefer_embedded(base_libraries, embedded_libraries)
    }

    fn rewrite_upstream(endpoints: &Self::Endpoints, raw_url: &str) -> Result<String> {
        endpoints.rewrite_upstream(raw_url)
    }

    fn default_artifact_url(endpoints: &Self::Endpoints, artifact_path: &str) -> Result<String> {
        endpoints.artifact_url(artifact_path)
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
