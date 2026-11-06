mod archive;
mod artifact;
mod driver;
mod family;
mod libraries;
mod processor;
mod profile;
mod source;

pub use archive::InstallerArchive;
pub use artifact::{
    InstallerArtifact, build_installer_artifact, installer_coordinate_file_name,
    installer_coordinate_path,
};
pub use driver::InstallerFamilyDriver;
pub use family::{
    InstallerArtifactEndpoints, InstallerArtifactSource, InstallerFamily,
    InstallerFamilyInstallStatus, InstallerFamilyRemoteResolver, PreparedInstallerVersion,
    ResolvedInstallerFamilyLaunchVersion, ResolvedInstallerFamilyMetadata,
    ResolvedInstallerFamilyVersion,
};
pub use libraries::{
    is_retro_futura_runtime_library, merge_libraries_prefer_embedded, merge_retro_futura_libraries,
    normalize_library_urls,
};
pub use processor::{
    installer_client_processors_ready, installer_has_client_processors,
    run_installer_client_processors, submit_installer_profile_libraries,
};
pub use profile::{
    InstallerInstallStatus, InstallerLaunchVersionRequest, InstallerPersistedState,
    load_persisted_installer_state, persist_installer_launch_version, prepare_installer_state,
};
pub(crate) use profile::{
    InstallerStatePaths, installer_install_status, profile_game_and_raw_loader_version,
    resolve_installer_processor_runtime, validate_installer_profile_identity,
};
pub use source::{
    InstallerMavenArtifactSpec, InstallerMavenEndpoints, InstallerMavenSource, MavenEndpoints,
    MavenSpec,
};
