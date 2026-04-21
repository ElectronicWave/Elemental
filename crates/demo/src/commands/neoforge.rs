use anyhow::Result;
use elemental::driver::drivers::neoforge::{config::NeoForgeLaunchConfig, driver::NeoForgeDriver};

use crate::{commands::run_loader_demo, config::DemoConfig};

pub async fn run(config: DemoConfig) -> Result<()> {
    let driver = NeoForgeDriver::with_defaults()?;

    run_loader_demo(
        config,
        "neoforge",
        &driver,
        |driver, instance, game_version, loader_version, launch_config: &NeoForgeLaunchConfig| {
            Box::pin(async move {
                driver
                    .prepare_with_config(instance, game_version, loader_version, launch_config)
                    .await
            })
        },
        |driver, authorizer, prepared, launch_config: &NeoForgeLaunchConfig| {
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
