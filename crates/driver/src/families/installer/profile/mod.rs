mod merge;
mod state;

use std::path::Path;

use elemental_core::{
    minecraft::MinecraftVersionId,
    runtime::{RuntimeValidationMode, resolve_runtime},
    storage::Storage,
};
use elemental_infra::downloader::{DownloadBatch, DownloadSession};
use elemental_schema::{forge::ForgeInstallerProfile, mojang::piston::PistonMetaLibraries};

use crate::{
    drivers::vanilla::source::{VanillaSource, resolve_vanilla_metadata},
    error::{Error, Result},
    families::{
        installer::{
            InstallerArchive, InstallerArtifact, installer_client_processors_ready,
            libraries::installer_library_artifacts,
        },
        version_json::{
            PreparedVersionJsonInstance, ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata,
            VersionJsonInstanceLayout, VersionJsonRemoteResolver, VersionJsonRootLayout,
            storage::library_artifacts_present,
        },
    },
    loader_version::LoaderVersionId,
};

use merge::merge_embedded_version;
pub use state::InstallerPersistedState;
pub(crate) use state::InstallerStatePaths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerInstallStatus {
    pub installer_downloaded: bool,
    pub install_profile_persisted: bool,
    pub embedded_version_persisted: bool,
    pub profile_libraries_ready: bool,
    pub processors_completed: bool,
    pub launch_version_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstallerProfileIdentity {
    game_version: String,
    loader_version: LoaderVersionId,
}

impl InstallerProfileIdentity {
    fn from_profile(
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
            .and_then(|coordinate| coordinate.split(':').nth(2).map(ToOwned::to_owned))
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

pub async fn prepare_installer_state<V>(
    session: &DownloadSession,
    installer_artifact: &InstallerArtifact,
    instance_root: &Path,
    libraries_root: &Path,
    family_name: &str,
    validate_profile: V,
) -> Result<InstallerPersistedState>
where
    V: Fn(&ForgeInstallerProfile) -> Result<()>,
{
    session
        .submit(DownloadBatch::new(vec![installer_artifact.download_task()]).group("installer"))
        .await
        .map_err(|source| Error::DownloadInstaller {
            family_name: family_name.to_owned(),
            source,
        })?;
    session
        .wait_idle()
        .await
        .map_err(|source| Error::DownloadInstaller {
            family_name: family_name.to_owned(),
            source,
        })?;

    read_installer_state(
        installer_artifact,
        instance_root,
        libraries_root,
        family_name,
        validate_profile,
    )
    .await
}

async fn read_installer_state<V>(
    installer_artifact: &InstallerArtifact,
    instance_root: &Path,
    libraries_root: &Path,
    family_name: &str,
    validate_profile: V,
) -> Result<InstallerPersistedState>
where
    V: Fn(&ForgeInstallerProfile) -> Result<()>,
{
    let archive = InstallerArchive::new(installer_artifact.path.clone());
    let state_paths = InstallerStatePaths::new(instance_root, family_name);
    let install_profile = archive
        .read_json::<ForgeInstallerProfile>("install_profile.json")
        .map_err(|source| Error::ReadInstallerProfile {
            family_name: family_name.to_owned(),
            source: Box::new(source),
        })?;
    validate_profile(&install_profile)?;
    let embedded_version = state::read_embedded_version(&archive, &install_profile, family_name)?;

    state_paths
        .persist_install_profile(&install_profile)
        .await?;
    state_paths
        .persist_embedded_version(embedded_version.as_ref())
        .await?;
    archive
        .extract_maven_artifacts(libraries_root)
        .map_err(|source| Error::ExtractInstallerMavenArtifacts {
            family_name: family_name.to_owned(),
            source: Box::new(source),
        })?;

    Ok(InstallerPersistedState {
        install_profile,
        embedded_version,
    })
}

pub async fn persist_installer_launch_version<RR, L, VL, F, M>(
    request: InstallerLaunchVersionRequest<'_, RR, L, VL, F, M>,
) -> Result<ResolvedVersionJsonInstance<RR, L, VL>>
where
    RR: VersionJsonRemoteResolver + Clone,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
    F: Fn(Vec<PistonMetaLibraries>) -> Result<Vec<PistonMetaLibraries>>,
    M: Fn(Vec<PistonMetaLibraries>, Vec<PistonMetaLibraries>) -> Vec<PistonMetaLibraries>,
{
    let embedded_version =
        request
            .embedded_version
            .ok_or_else(|| Error::InstallerEmbeddedVersionMissing {
                family_name: request.family_name.to_owned(),
            })?;
    let base_metadata =
        resolve_vanilla_metadata(request.vanilla_source, request.game_version.as_str()).await?;
    let merged_metadata = merge_embedded_version(
        base_metadata.metadata,
        embedded_version,
        request.normalize_libraries,
        request.merge_libraries,
        request.family_name,
    )?;
    ResolvedVersionJsonMetadata::new(
        request.remote_resolver.clone(),
        merged_metadata,
        base_metadata.asset_index_objects,
    )
    .persist(request.instance)
    .await
}

pub struct InstallerLaunchVersionRequest<'a, RR, L, VL, F, M>
where
    RR: VersionJsonRemoteResolver + Clone,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
    F: Fn(Vec<PistonMetaLibraries>) -> Result<Vec<PistonMetaLibraries>>,
    M: Fn(Vec<PistonMetaLibraries>, Vec<PistonMetaLibraries>) -> Vec<PistonMetaLibraries>,
{
    pub instance: &'a Storage<VL, Storage<L>>,
    pub game_version: &'a MinecraftVersionId,
    pub remote_resolver: &'a RR,
    pub vanilla_source: &'a VanillaSource,
    pub embedded_version: Option<&'a serde_json::Value>,
    pub normalize_libraries: F,
    pub merge_libraries: M,
    pub family_name: &'a str,
}

pub fn load_persisted_installer_state(
    instance_root: &Path,
    family_name: &str,
) -> Result<InstallerPersistedState> {
    InstallerStatePaths::new(instance_root, family_name).load()
}

pub(crate) fn profile_game_and_raw_loader_version(
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
    loader_name: &str,
) -> Result<(String, LoaderVersionId)> {
    let identity =
        InstallerProfileIdentity::from_profile(install_profile, family_name, loader_name)?;
    Ok((identity.game_version, identity.loader_version))
}

pub(crate) fn validate_installer_profile_identity(
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

fn profile_libraries_ready<L, VL>(
    instance: &Storage<VL, Storage<L>>,
    install_profile: &ForgeInstallerProfile,
) -> Result<bool>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let artifact_paths = install_profile
        .libraries
        .iter()
        .flat_map(installer_library_artifacts)
        .map(|artifact| artifact.path.as_str().into());
    library_artifacts_present(&instance.parent, artifact_paths)
}

pub(crate) async fn resolve_installer_processor_runtime<L, VL>(
    launch_version: &PreparedVersionJsonInstance<L, VL>,
    runtime_executable_path: Option<&Path>,
    runtime_validation: RuntimeValidationMode,
    operation_name: &str,
) -> Result<elemental_core::runtime::distribution::Distribution>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    Ok(resolve_runtime(
        launch_version.required_java_major_version(),
        runtime_executable_path,
        runtime_validation,
        operation_name,
    )
    .await?)
}

pub(crate) async fn installer_install_status<L, VL>(
    instance: &Storage<VL, Storage<L>>,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
    launch_version_ready: bool,
    family_name: &str,
) -> Result<InstallerInstallStatus>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let state_paths = InstallerStatePaths::new(&instance.path, family_name);

    Ok(InstallerInstallStatus {
        installer_downloaded: installer_artifact.path.exists(),
        install_profile_persisted: state_paths.install_profile_path().exists(),
        embedded_version_persisted: state_paths.embedded_version_path().exists(),
        profile_libraries_ready: profile_libraries_ready(instance, install_profile)?,
        processors_completed: installer_client_processors_ready(
            instance,
            installer_artifact,
            install_profile,
            family_name,
        )?,
        launch_version_ready,
    })
}
