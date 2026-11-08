use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use elemental::{
    core::minecraft::MinecraftVersionId,
    driver::{
        descriptors::VANILLA_DRIVER, driver::DriverDescriptor, loader_version::LoaderVersionId,
    },
    launcher::{DriverSpec, LaunchOptions},
};

#[derive(Clone)]
pub struct LaunchpadConfig {
    pub spec: DriverSpec,
    pub options: LaunchOptions,
    pub runtime_paths: Vec<PathBuf>,
    pub storage_root: PathBuf,
    pub instance_name: String,
    pub local_only: bool,
}

#[derive(Clone)]
pub enum LaunchpadCommand {
    Launch(Box<LaunchpadConfig>),
    ListInstances { storage_root: PathBuf },
    Exit,
}

pub fn build_spec(
    driver: DriverDescriptor,
    game_version: MinecraftVersionId,
    loader_version: Option<LoaderVersionId>,
) -> Result<DriverSpec> {
    if driver == VANILLA_DRIVER {
        if loader_version.is_some() {
            bail!("vanilla does not accept a loader version");
        }
        return Ok(DriverSpec::vanilla(game_version));
    }

    let loader_version = loader_version
        .with_context(|| format!("{} launchpad requires a loader version", driver.name))?;
    Ok(DriverSpec::any(driver, game_version, loader_version)?)
}
