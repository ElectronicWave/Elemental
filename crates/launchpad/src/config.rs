use std::path::PathBuf;

use anyhow::Result;
use elemental::{
    core::minecraft::MinecraftVersionId,
    driver::{driver::DriverDescriptor, loader_version::LoaderVersionId},
    launcher::{DriverSpec, LaunchOptions, RuntimeOptions},
};

#[derive(Clone)]
pub struct LaunchpadConfig {
    pub spec: DriverSpec,
    pub runtime: RuntimeOptions,
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
    Ok(DriverSpec::from_descriptor(
        driver,
        game_version,
        loader_version,
    )?)
}
