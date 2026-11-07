mod client;
mod context;
mod libraries;
mod runner;
mod variables;

use elemental_schema::forge::{ForgeInstallerProcessor, ForgeInstallerProfile};

pub use client::{installer_client_processors_ready, run_installer_client_processors};
pub use libraries::submit_installer_profile_libraries;

pub fn installer_has_client_processors(install_profile: &ForgeInstallerProfile) -> bool {
    install_profile
        .processors
        .iter()
        .any(processor_applies_to_client)
}

fn processor_applies_to_client(processor: &ForgeInstallerProcessor) -> bool {
    processor.sides.is_empty() || processor.sides.iter().any(|side| side == "client")
}
