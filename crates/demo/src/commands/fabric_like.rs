use std::time::Instant;

use anyhow::{Result, bail};
use elemental::driver::drivers::fabric::{
    config::FabricLaunchConfig, driver::FabricDriver, source::FabricFlavor,
};

use crate::{
    commands::{
        build_launch_config, ensure_instance, finalize_launch, offline_authorizer,
        require_loader_version,
    },
    config::{DemoConfig, DemoDriver},
};

pub async fn run(config: DemoConfig) -> Result<()> {
    let loader_version = require_loader_version(&config, "fabric-like")?;
    let instance = ensure_instance(&config).await?;
    let driver = FabricDriver::for_flavor(fabric_flavor(config.driver)?)?;
    let launch_config: FabricLaunchConfig = build_launch_config(&config);

    let started_at = Instant::now();
    let prepared = driver
        .prepare(
            &instance,
            config.game_version.clone(),
            loader_version.clone(),
        )
        .await?;
    let prepare_elapsed = started_at.elapsed();
    let (runtime, command) = driver
        .build_launch_command(offline_authorizer(), &prepared, &launch_config)
        .await?;
    finalize_launch(
        &config,
        Some(loader_version.as_str()),
        prepare_elapsed.as_millis(),
        &prepared.install_status,
        &prepared.resolved_version.version,
        runtime,
        command,
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
