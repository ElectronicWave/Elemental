use std::path::PathBuf;

use elemental::core::runtime::RuntimeValidationMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemoDriver {
    Vanilla,
    Fabric,
    LegacyFabric,
    Babric,
    Quilt,
    Forge,
    Cleanroom,
    NeoForge,
}

#[derive(Clone, Debug)]
pub struct DemoConfig {
    pub driver: DemoDriver,
    pub storage_root: PathBuf,
    pub instance_name: String,
    pub game_version: String,
    pub loader_version: Option<String>,
    pub runtime_major_version: Option<usize>,
    pub runtime_paths: Vec<PathBuf>,
    pub runtime_executable_path: Option<PathBuf>,
    pub runtime_validation: RuntimeValidationMode,
}

impl DemoDriver {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Fabric => "Fabric",
            Self::LegacyFabric => "LegacyFabric",
            Self::Babric => "Babric",
            Self::Quilt => "Quilt",
            Self::Forge => "Forge",
            Self::Cleanroom => "Cleanroom",
            Self::NeoForge => "NeoForge",
        }
    }
}
