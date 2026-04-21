use anyhow::{Result, bail};
use elemental::driver::drivers::fabric::{
    config::FabricLaunchConfig, driver::FabricDriver, source::FabricFlavor,
};

use crate::{
    commands::run_loader_demo,
    config::{DemoConfig, DemoDriver},
};

pub async fn run(config: DemoConfig) -> Result<()> {
    let driver_kind = config.driver;
    let driver = FabricDriver::for_flavor(fabric_flavor(driver_kind)?)?;

    run_loader_demo(
        config,
        "fabric-like",
        &driver,
        |driver, instance, game_version, loader_version, _launch_config: &FabricLaunchConfig| {
            Box::pin(async move { driver.prepare(instance, game_version, loader_version).await })
        },
        |driver, authorizer, prepared, launch_config: &FabricLaunchConfig| {
            Box::pin(async move {
                driver
                    .build_launch_command(authorizer, prepared, launch_config)
                    .await
            })
        },
        |prepared| &prepared.install_status,
        |prepared| &prepared.resolved_version.version,
    )
    .await
}

fn fabric_flavor(driver: DemoDriver) -> Result<FabricFlavor> {
    match driver {
        DemoDriver::Fabric => Ok(FabricFlavor::Fabric),
        DemoDriver::LegacyFabric => Ok(FabricFlavor::LegacyFabric),
        DemoDriver::Babric => Ok(FabricFlavor::Babric),
        DemoDriver::Vanilla | DemoDriver::Quilt | DemoDriver::Forge | DemoDriver::NeoForge => {
            bail!("unsupported fabric-like demo driver: {}", driver.as_str())
        }
    }
}
