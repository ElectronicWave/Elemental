use std::path::PathBuf;

use elemental::launcher::{DriverSpec, LaunchOptions, RuntimeOptions};

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
