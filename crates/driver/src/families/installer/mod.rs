mod archive;
mod artifact;
mod family;
mod libraries;
mod processor;
mod profile;

pub use archive::InstallerArchive;
pub use artifact::{
    InstallerArtifact, build_installer_artifact, installer_coordinate_file_name,
    installer_coordinate_path,
};
pub use family::{
    InstallerFamily, InstallerFamilyInstallStatus, InstallerFamilyRemoteResolver,
    PreparedInstallerFamilyLaunchVersion, PreparedInstallerFamilyVersion,
    ResolvedInstallerFamilyLaunchVersion, ResolvedInstallerFamilyMetadata,
    ResolvedInstallerFamilyVersion,
};
pub use libraries::normalize_library_urls;
pub use processor::{
    ensure_installer_profile_libraries_downloaded, installer_client_processors_ready,
    run_installer_client_processors,
};
pub use profile::{
    InstallerInstallStatus, InstallerLaunchVersionRequest, InstallerPersistedState,
    embedded_version_path, ensure_installer_artifact_downloaded, install_profile_path,
    installer_install_status, load_persisted_installer_state, persist_embedded_version,
    persist_install_profile, prepare_installer_launch_version, prepare_installer_state,
    profile_game_and_raw_loader_version, profile_libraries_ready, read_embedded_version,
    resolve_installer_processor_runtime, validate_installer_profile_identity,
};
