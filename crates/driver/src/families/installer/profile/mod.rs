mod identity;
mod install;
mod launch;
mod merge;
mod state;
mod status;

pub use identity::{profile_game_and_raw_loader_version, validate_installer_profile_identity};
pub use install::prepare_installer_state;
pub use launch::{
    InstallerLaunchVersionRequest, load_persisted_installer_state, persist_installer_launch_version,
};
pub use state::InstallerPersistedState;
pub(crate) use state::InstallerStatePaths;
pub use status::{installer_install_status, resolve_installer_processor_runtime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerInstallStatus {
    pub installer_downloaded: bool,
    pub install_profile_persisted: bool,
    pub embedded_version_persisted: bool,
    pub profile_libraries_ready: bool,
    pub processors_completed: bool,
    pub launch_version_ready: bool,
}
