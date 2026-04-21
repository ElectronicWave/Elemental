use std::sync::Arc;

use anyhow::Result;
use elemental::driver::drivers::neoforge::{config::NeoForgeLaunchConfig, driver::NeoForgeDriver};

use crate::{commands::run_loader_demo, config::DemoConfig};

pub async fn run(config: DemoConfig) -> Result<()> {
    let driver = Arc::new(NeoForgeDriver::with_defaults()?);
    let prepare_driver = driver.clone();
    let build_driver = driver.clone();

    run_loader_demo(
        config,
        "neoforge",
        move |instance, game_version, loader_version, launch_config: &NeoForgeLaunchConfig| {
            let driver = prepare_driver.clone();
            Box::pin(async move {
                driver
                    .prepare_with_config(instance, game_version, loader_version, launch_config)
                    .await
            })
        },
        move |authorizer, prepared, launch_config: &NeoForgeLaunchConfig| {
            let driver = build_driver.clone();
            Box::pin(async move {
                driver
                    .build_launch_command(authorizer, prepared, launch_config)
                    .await
            })
        },
        |prepared| &prepared.install_status,
        |prepared| &prepared.launch_version.resolved_version.version,
    )
    .await
}
