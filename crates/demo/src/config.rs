use std::path::PathBuf;

use elemental::core::{minecraft::MinecraftVersionId, runtime::RuntimeValidationMode};
use elemental::driver::{
    descriptors::{
        BABRIC_DRIVER, CLEANROOM_DRIVER, FABRIC_DRIVER, FORGE_DRIVER, LEGACY_FABRIC_DRIVER,
        LITELOADER_DRIVER, NEOFORGE_DRIVER, OUTLANDS_DRIVER, QUILT_DRIVER, RIFT_DRIVER,
        VANILLA_DRIVER,
    },
    driver::DriverDescriptor,
    loader_version::LoaderVersionId,
};

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
    Outlands,
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

#[derive(Clone, Debug)]
pub enum DemoCommand {
    Launch(DemoConfig),
    ListInstances { storage_root: PathBuf },
}

impl DemoDriver {
    pub fn descriptor(self) -> DriverDescriptor {
        match self {
            Self::Vanilla => VANILLA_DRIVER,
            Self::Fabric => FABRIC_DRIVER,
            Self::LegacyFabric => LEGACY_FABRIC_DRIVER,
            Self::Babric => BABRIC_DRIVER,
            Self::Quilt => QUILT_DRIVER,
            Self::LiteLoader => LITELOADER_DRIVER,
            Self::Rift => RIFT_DRIVER,
            Self::Forge => FORGE_DRIVER,
            Self::Cleanroom => CLEANROOM_DRIVER,
            Self::Outlands => OUTLANDS_DRIVER,
            Self::NeoForge => NEOFORGE_DRIVER,
        }
    }

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
            Self::Outlands => "Outlands",
            Self::NeoForge => "NeoForge",
        }
    }
}
