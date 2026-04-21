use anyhow::Result;
use elemental::driver::drivers::vanilla::{config::VanillaLaunchConfig, driver::VanillaDriver};

use crate::{
    commands::{
        build_launch_config, ensure_instance, finalize_launch, offline_authorizer, time_operation,
    },
    config::DemoConfig,
};

pub async fn run(config: DemoConfig) -> Result<()> {
    let instance = ensure_instance(&config).await?;
    let driver = VanillaDriver::with_defaults()?;
    let launch_config: VanillaLaunchConfig = build_launch_config(&config);

    let (prepared, prepare_elapsed) =
        time_operation(driver.prepare(&instance, config.game_version.clone())).await?;
    let (runtime, command) = driver
        .build_launch_command(offline_authorizer(), &prepared, &launch_config)
        .await?;

    finalize_launch(
        &config,
        None,
        prepare_elapsed.as_millis(),
        &prepared.install_status,
        &prepared.resolved_version.version,
        runtime,
        command,
    )
    .await
}
