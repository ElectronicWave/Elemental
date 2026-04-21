use anyhow::Result;
use elemental::driver::drivers::quilt::{config::QuiltLaunchConfig, driver::QuiltDriverFamily};

use crate::{commands::run_loader_demo, config::DemoConfig};

pub async fn run(config: DemoConfig) -> Result<()> {
    let driver = QuiltDriverFamily::new_driver_with_defaults()?;

    run_loader_demo(
        config,
        "quilt",
        &driver,
        |driver, instance, game_version, loader_version, _launch_config: &QuiltLaunchConfig| {
            Box::pin(async move { driver.prepare(instance, game_version, loader_version).await })
        },
        |driver, authorizer, prepared, launch_config: &QuiltLaunchConfig| {
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
