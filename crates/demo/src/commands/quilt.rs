use std::time::Instant;

use anyhow::Result;
use elemental::driver::drivers::quilt::{config::QuiltLaunchConfig, driver::QuiltDriver};

use crate::{
    commands::{
        build_launch_config, ensure_instance, finalize_launch, offline_authorizer,
        require_loader_version,
    },
    config::DemoConfig,
};

pub async fn run(config: DemoConfig) -> Result<()> {
    let loader_version = require_loader_version(&config, "quilt")?;
    let instance = ensure_instance(&config).await?;
    let driver = QuiltDriver::with_defaults()?;
    let launch_config: QuiltLaunchConfig = build_launch_config(&config);

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
