use std::time::Instant;

use anyhow::{Context, Result, bail};
use elemental::{
    core::{auth::authorizers::offline::OfflineAuthorizer, storage::Storage},
    driver::{
        drivers::fabric::{config::FabricLaunchConfig, driver::FabricDriver, source::FabricFlavor},
        families::version_json::{BaseLayout, VersionJsonGameStorageExt},
    },
};

use crate::{
    config::{DemoConfig, DemoDriver},
    diagnostics::{
        collect_version_diagnostics, print_launch_diagnostics, print_summary, run_logged_process,
    },
};

pub async fn run(config: DemoConfig) -> Result<()> {
    let loader_version = config
        .loader_version
        .clone()
        .context("fabric-like demo requires a loader version")?;
    let storage = Storage::new(config.storage_root.clone(), BaseLayout);
    let instance = storage
        .ensure_instance(config.instance_name.clone(), BaseLayout)
        .await?;
    let driver = FabricDriver::for_flavor(fabric_flavor(config.driver)?)?;
    let launch_config = FabricLaunchConfig::new();
    let authorizer = OfflineAuthorizer {
        username: "Player".to_owned(),
    };

    let started_at = Instant::now();
    let prepared = driver
        .prepare(
            &instance,
            config.game_version.clone(),
            loader_version.clone(),
        )
        .await?;
    let prepare_elapsed = started_at.elapsed();

    let diagnostics = collect_version_diagnostics(&prepared.resolved_version.version)?;
    let install_status = prepared.install_status.clone();
    let (runtime, command) = driver
        .build_launch_command(authorizer, &prepared, &launch_config)
        .await?;
    let runtime_executable = runtime.executable().to_path_buf();

    print_launch_diagnostics(
        config.driver.as_str(),
        Some(loader_version.as_str()),
        &config.instance_name,
        &config.game_version,
        prepare_elapsed.as_millis(),
        &install_status,
        runtime_executable.as_path(),
        &diagnostics,
        &command,
    );

    let exit_status = run_logged_process(command).await?;
    print_summary(
        config.driver.as_str(),
        &config.game_version,
        Some(loader_version.as_str()),
        runtime_executable.as_path(),
        diagnostics.version_root.as_path(),
        &install_status,
        prepare_elapsed.as_millis(),
        exit_status,
    );

    Ok(())
}

fn fabric_flavor(driver: DemoDriver) -> Result<FabricFlavor> {
    match driver {
        DemoDriver::Fabric => Ok(FabricFlavor::Fabric),
        DemoDriver::LegacyFabric => Ok(FabricFlavor::LegacyFabric),
        DemoDriver::Babric => Ok(FabricFlavor::Babric),
        DemoDriver::Vanilla | DemoDriver::Quilt => {
            bail!("unsupported fabric-like demo driver: {}", driver.as_str())
        }
    }
}
