use elemental_core::minecraft::MinecraftVersionId;
use elemental_driver::descriptors::{
    BABRIC_DRIVER, CLEANROOM_DRIVER, FABRIC_DRIVER, FORGE_DRIVER, LEGACY_FABRIC_DRIVER,
    LITELOADER_DRIVER, NEOFORGE_DRIVER, OUTLANDS_DRIVER, QUILT_DRIVER, RIFT_DRIVER, VANILLA_DRIVER,
};
use elemental_driver::driver::DriverDescriptor;
use elemental_driver::loader_version::LoaderVersionId;

use crate::error::{Error, Result};

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

impl DriverSpec {
    pub fn vanilla(game_version: MinecraftVersionId) -> Self {
        Self::Vanilla(VanillaSpec { game_version })
    }

    pub fn from_descriptor(
        descriptor: DriverDescriptor,
        game_version: MinecraftVersionId,
        loader_version: Option<LoaderVersionId>,
    ) -> Result<Self> {
        if descriptor == VANILLA_DRIVER {
            if loader_version.is_some() {
                return Err(Error::UnexpectedLoaderVersion {
                    id: descriptor.id.to_owned(),
                });
            }
            return Ok(Self::vanilla(game_version));
        }

        let loader_version = loader_version.ok_or_else(|| Error::LoaderVersionRequired {
            id: descriptor.id.to_owned(),
        })?;
        Self::any(descriptor, game_version, loader_version)
    }

    pub fn any(
        loader: DriverDescriptor,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<Self> {
        if loader == VANILLA_DRIVER {
            return Err(Error::UnexpectedLoaderVersion {
                id: loader.id.to_owned(),
            });
        }

        let spec = LoaderSpec {
            game_version,
            loader_version,
        };

        match loader {
            FABRIC_DRIVER => Ok(Self::Fabric(spec)),
            LEGACY_FABRIC_DRIVER => Ok(Self::LegacyFabric(spec)),
            BABRIC_DRIVER => Ok(Self::Babric(spec)),
            QUILT_DRIVER => Ok(Self::Quilt(spec)),
            LITELOADER_DRIVER => Ok(Self::LiteLoader(spec)),
            RIFT_DRIVER => Ok(Self::Rift(spec)),
            FORGE_DRIVER => Ok(Self::Forge(spec)),
            CLEANROOM_DRIVER => Ok(Self::Cleanroom(spec)),
            OUTLANDS_DRIVER => Ok(Self::Outlands(spec)),
            NEOFORGE_DRIVER => Ok(Self::NeoForge(spec)),
            _ => Err(Error::UnsupportedDriver {
                id: loader.id.to_owned(),
            }),
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
