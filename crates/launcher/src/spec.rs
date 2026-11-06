use elemental_core::minecraft::MinecraftVersionId;
use elemental_driver::descriptors::{
    BABRIC_DRIVER, CLEANROOM_DRIVER, FABRIC_DRIVER, FORGE_DRIVER, LEGACY_FABRIC_DRIVER,
    LITELOADER_DRIVER, NEOFORGE_DRIVER, OUTLANDS_DRIVER, QUILT_DRIVER, RIFT_DRIVER, VANILLA_DRIVER,
};
use elemental_driver::driver::DriverDescriptor;
use elemental_driver::loader_version::LoaderVersionId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VanillaSpec {
    pub game_version: MinecraftVersionId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderSpec {
    pub game_version: MinecraftVersionId,
    pub loader_version: LoaderVersionId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverSpec {
    Vanilla(VanillaSpec),
    Fabric(LoaderSpec),
    LegacyFabric(LoaderSpec),
    Babric(LoaderSpec),
    Quilt(LoaderSpec),
    LiteLoader(LoaderSpec),
    Rift(LoaderSpec),
    Forge(LoaderSpec),
    Cleanroom(LoaderSpec),
    Outlands(LoaderSpec),
    NeoForge(LoaderSpec),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DriverFlow {
    Vanilla,
    Profiled,
    Installer,
}

impl DriverSpec {
    pub(crate) fn flow(&self) -> DriverFlow {
        match self {
            Self::Vanilla(_) => DriverFlow::Vanilla,
            Self::Fabric(_)
            | Self::LegacyFabric(_)
            | Self::Babric(_)
            | Self::Quilt(_)
            | Self::LiteLoader(_)
            | Self::Rift(_) => DriverFlow::Profiled,
            Self::Forge(_) | Self::Cleanroom(_) | Self::Outlands(_) | Self::NeoForge(_) => {
                DriverFlow::Installer
            }
        }
    }

    pub fn descriptor(&self) -> DriverDescriptor {
        match self {
            Self::Vanilla(_) => VANILLA_DRIVER,
            Self::Fabric(_) => FABRIC_DRIVER,
            Self::LegacyFabric(_) => LEGACY_FABRIC_DRIVER,
            Self::Babric(_) => BABRIC_DRIVER,
            Self::Quilt(_) => QUILT_DRIVER,
            Self::LiteLoader(_) => LITELOADER_DRIVER,
            Self::Rift(_) => RIFT_DRIVER,
            Self::Forge(_) => FORGE_DRIVER,
            Self::Cleanroom(_) => CLEANROOM_DRIVER,
            Self::Outlands(_) => OUTLANDS_DRIVER,
            Self::NeoForge(_) => NEOFORGE_DRIVER,
        }
    }

    pub fn id(&self) -> &'static str {
        self.descriptor().id
    }

    pub fn name(&self) -> &'static str {
        self.descriptor().name
    }

    pub fn game_version(&self) -> &MinecraftVersionId {
        match self {
            Self::Vanilla(spec) => &spec.game_version,
            Self::Fabric(spec)
            | Self::LegacyFabric(spec)
            | Self::Babric(spec)
            | Self::Quilt(spec)
            | Self::LiteLoader(spec)
            | Self::Rift(spec)
            | Self::Forge(spec)
            | Self::Cleanroom(spec)
            | Self::Outlands(spec)
            | Self::NeoForge(spec) => &spec.game_version,
        }
    }

    pub fn loader_version(&self) -> Option<&LoaderVersionId> {
        match self {
            Self::Vanilla(_) => None,
            Self::Fabric(spec)
            | Self::LegacyFabric(spec)
            | Self::Babric(spec)
            | Self::Quilt(spec)
            | Self::LiteLoader(spec)
            | Self::Rift(spec)
            | Self::Forge(spec)
            | Self::Cleanroom(spec)
            | Self::Outlands(spec)
            | Self::NeoForge(spec) => Some(&spec.loader_version),
        }
    }
}
