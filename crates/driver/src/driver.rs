use anyhow::Result;
use async_trait::async_trait;
use elemental_core::{minecraft::MinecraftVersionId, storage::layout::Layout};

use crate::inspect::InstanceProbe;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverDescriptor {
    pub id: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledDriver {
    pub driver: DriverDescriptor,
    pub driver_version: Option<String>,
    pub game_version: Option<MinecraftVersionId>,
    pub description: Option<String>,
}

#[async_trait]
pub trait Driver<L: Layout, VL: Layout> {
    fn descriptor(&self) -> DriverDescriptor;

    async fn inspect(&self, probe: &InstanceProbe<L, VL>) -> Result<Option<InstalledDriver>>;
}
