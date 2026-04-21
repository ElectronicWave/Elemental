use std::time::Instant;

use anyhow::{Context, Result};
use elemental::{
    core::{auth::authorizers::offline::OfflineAuthorizer, storage::Storage},
    driver::{
        drivers::neoforge::{config::NeoForgeLaunchConfig, driver::NeoForgeDriver},
        families::version_json::{BaseLayout, VersionJsonGameStorageExt},
    },
};

use crate::{
    config::DemoConfig,
    diagnostics::{
        LaunchDiagnostics, LaunchSummary, collect_version_diagnostics, print_launch_diagnostics,
        print_summary, run_logged_process,
    },
};

pub async fn run(config: DemoConfig) -> Result<()> {
    let loader_version = config
        .loader_version
        .clone()
        .context("neoforge demo requires a loader version")?;
    let storage = Storage::new(config.storage_root.clone(), BaseLayout);
    let instance = storage
        .ensure_instance(config.instance_name.clone(), BaseLayout)
        .await?;
    let driver = NeoForgeDriver::with_defaults()?;
    let mut launch_config = NeoForgeLaunchConfig::new();
    launch_config.runtime_major_version = config.runtime_major_version;
    launch_config.runtime_executable_path = config.runtime_executable_path.clone();
    let authorizer = OfflineAuthorizer {
        username: "Player".to_owned(),
    };

    let started_at = Instant::now();
    let prepared = driver
        .prepare_with_config(
            &instance,
            config.game_version.clone(),
            loader_version.clone(),
            &launch_config,
        )
        .await?;
    let prepare_elapsed = started_at.elapsed();

    let diagnostics =
        collect_version_diagnostics(&prepared.launch_version.resolved_version.version)?;
    let (runtime, command) = driver
        .build_launch_command(authorizer, &prepared, &launch_config)
        .await?;
    let runtime_executable = runtime.executable().to_path_buf();

    print_launch_diagnostics(&LaunchDiagnostics {
        driver_name: config.driver.as_str(),
        loader_version: Some(loader_version.as_str()),
        instance_name: &config.instance_name,
        game_version: &config.game_version,
        prepared_ms: prepare_elapsed.as_millis(),
        install_status: &prepared.install_status,
        runtime_executable: runtime_executable.as_path(),
        diagnostics: &diagnostics,
        command: &command,
    });

    let exit_status = run_logged_process(command).await?;
    print_summary(&LaunchSummary {
        driver_name: config.driver.as_str(),
        game_version: &config.game_version,
        loader_version: Some(loader_version.as_str()),
        runtime_executable: runtime_executable.as_path(),
        version_root: diagnostics.version_root.as_path(),
        install_status: &prepared.install_status,
        prepared_ms: prepare_elapsed.as_millis(),
        exit_status,
    });

    Ok(())
}
