mod fabric_like;
mod forge;
mod neoforge;
mod quilt;
mod vanilla;

use anyhow::Result;

use crate::config::{DemoConfig, DemoDriver};

pub async fn run(config: DemoConfig) -> Result<()> {
    match config.driver {
        DemoDriver::Vanilla => vanilla::run(config).await,
        DemoDriver::Fabric | DemoDriver::LegacyFabric | DemoDriver::Babric => {
            fabric_like::run(config).await
        }
        DemoDriver::Quilt => quilt::run(config).await,
        DemoDriver::Forge => forge::run(config).await,
        DemoDriver::NeoForge => neoforge::run(config).await,
    }
}
