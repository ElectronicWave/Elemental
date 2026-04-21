use std::sync::Arc;

use anyhow::Result;
use elemental::driver::drivers::quilt::{config::QuiltLaunchConfig, driver::QuiltDriver};

use crate::{commands::run_loader_demo, config::DemoConfig};

pub async fn run(config: DemoConfig) -> Result<()> {
    let driver = Arc::new(QuiltDriver::with_defaults()?);
    let prepare_driver = driver.clone();
    let build_driver = driver.clone();

    run_loader_demo(
        config,
        "quilt",
        move |instance, game_version, loader_version, launch_config: &QuiltLaunchConfig| {
            let _ = launch_config;
            let driver = prepare_driver.clone();
            Box::pin(async move { driver.prepare(instance, game_version, loader_version).await })
        },
        move |authorizer, prepared, launch_config: &QuiltLaunchConfig| {
            let driver = build_driver.clone();
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
