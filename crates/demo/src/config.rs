use std::path::PathBuf;

use elemental::core::{minecraft::MinecraftVersionId, runtime::RuntimeValidationMode};
use elemental::driver::{driver::DriverDescriptor, loader_version::LoaderVersionId};

#[derive(Clone, Debug)]
pub struct DemoConfig {
    pub driver: DriverDescriptor,
    pub local_only: bool,
    pub storage_root: PathBuf,
    pub instance_name: String,
    pub game_version: MinecraftVersionId,
    pub loader_version: Option<LoaderVersionId>,
    pub runtime_major_version: Option<usize>,
    pub runtime_paths: Vec<PathBuf>,
    pub runtime_executable_path: Option<PathBuf>,
    pub runtime_validation: RuntimeValidationMode,
}

#[derive(Clone, Debug)]
pub enum DemoCommand {
    Launch(DemoConfig),
    ListInstances { storage_root: PathBuf },
}
