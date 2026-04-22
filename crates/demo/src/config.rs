use std::path::PathBuf;

use elemental::core::{minecraft::MinecraftVersionId, runtime::RuntimeValidationMode};
use elemental::driver::loader_version::LoaderVersionId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemoDriver {
    Vanilla,
    Fabric,
    LegacyFabric,
    Babric,
    Quilt,
    LiteLoader,
    Rift,
    Forge,
    Cleanroom,
    NeoForge,
}

#[derive(Clone, Debug)]
pub struct DemoConfig {
    pub driver: DemoDriver,
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

impl DemoDriver {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Fabric => "Fabric",
            Self::LegacyFabric => "LegacyFabric",
            Self::Babric => "Babric",
            Self::Quilt => "Quilt",
            Self::LiteLoader => "LiteLoader",
            Self::Rift => "Rift",
            Self::Forge => "Forge",
            Self::Cleanroom => "Cleanroom",
            Self::NeoForge => "NeoForge",
        }
    }
}
