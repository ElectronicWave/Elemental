use std::str::FromStr;

use async_trait::async_trait;
use elemental_core::{minecraft::MinecraftVersionId, storage::layout::Layout};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::descriptors::from_id;
use crate::error::{Error, Result};
use crate::inspect::InstanceProbe;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverDescriptor {
    pub id: &'static str,
    pub name: &'static str,
}

impl TryFrom<&str> for DriverDescriptor {
    type Error = Error;

    fn try_from(id: &str) -> Result<Self> {
        from_id(id).ok_or_else(|| Error::UnsupportedDriver { id: id.to_owned() })
    }
}

impl TryFrom<String> for DriverDescriptor {
    type Error = Error;

    fn try_from(id: String) -> Result<Self> {
        Self::try_from(id.as_str())
    }
}

impl FromStr for DriverDescriptor {
    type Err = Error;

    fn from_str(id: &str) -> Result<Self> {
        Self::try_from(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledDriver {
    pub driver: DriverDescriptor,
    pub driver_version: Option<String>,
    pub game_version: Option<MinecraftVersionId>,
    pub description: Option<String>,
}

impl InstalledDriver {
    pub fn version_json(
        descriptor: DriverDescriptor,
        metadata: &PistonMetaData,
        driver_version: Option<String>,
    ) -> Self {
        Self {
            driver: descriptor,
            driver_version,
            game_version: metadata
                .inherits_from
                .clone()
                .or_else(|| Some(metadata.id.clone()))
                .map(MinecraftVersionId::from),
            description: Some(metadata.release_type.clone()),
        }
    }
}

#[async_trait]
pub trait Driver<L: Layout, VL: Layout> {
    fn descriptor(&self) -> DriverDescriptor;

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>>;
}
