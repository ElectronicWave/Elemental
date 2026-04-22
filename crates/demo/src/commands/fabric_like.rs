use anyhow::{Result, bail};
use elemental::driver::drivers::fabric::{driver::FabricDriverFamily, source::FabricFlavor};

use crate::{
    commands::run_profiled_version_json_demo,
    config::{DemoConfig, DemoDriver},
};

pub async fn run(config: DemoConfig) -> Result<()> {
    let driver_kind = config.driver;
    let driver = FabricDriverFamily::new(fabric_flavor(driver_kind)?).new_driver_with_defaults()?;

    run_profiled_version_json_demo(config, "fabric-like", &driver).await
}

fn fabric_flavor(driver: DemoDriver) -> Result<FabricFlavor> {
    match driver {
        DemoDriver::Fabric => Ok(FabricFlavor::Fabric),
        DemoDriver::LegacyFabric => Ok(FabricFlavor::LegacyFabric),
        DemoDriver::Babric => Ok(FabricFlavor::Babric),
        DemoDriver::Vanilla
        | DemoDriver::Quilt
        | DemoDriver::Rift
        | DemoDriver::Forge
        | DemoDriver::Cleanroom
        | DemoDriver::NeoForge => {
            bail!("unsupported fabric-like demo driver: {}", driver.as_str())
        }
    }
}
