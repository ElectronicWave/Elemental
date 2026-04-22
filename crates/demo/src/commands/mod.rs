mod cleanroom;
mod fabric_like;
mod forge;
mod neoforge;
mod quilt;
mod vanilla;

use std::{fmt::Debug, future::Future, time::Duration};

use anyhow::{Context, Result};
use elemental::{
    core::{
        auth::authorizers::offline::OfflineAuthorizer, launcher::command::LaunchCommand,
        runtime::distribution::Distribution, storage::Storage,
    },
    driver::{
        drivers::vanilla::config::VanillaLaunchConfig,
        families::{
            installer::{InstallerFamilyDriver, InstallerFamilyDriverSpec},
            version_json::{
                BaseInstanceLayout, BaseRootLayout, ProfiledVersionJsonDriver,
                ProfiledVersionJsonFamily, VersionJsonGameStorageExt, VersionJsonInstanceLayout,
                VersionJsonRootLayout,
            },
        },
    },
};

use crate::config::{DemoConfig, DemoDriver};
use crate::diagnostics::{
    LaunchDiagnostics, LaunchSummary, collect_version_diagnostics, print_launch_diagnostics,
    print_summary, run_logged_process,
};

pub async fn run(config: DemoConfig) -> Result<()> {
    match config.driver {
        DemoDriver::Vanilla => vanilla::run(config).await,
        DemoDriver::Fabric | DemoDriver::LegacyFabric | DemoDriver::Babric => {
            fabric_like::run(config).await
        }
        DemoDriver::Quilt => quilt::run(config).await,
        DemoDriver::Forge => forge::run(config).await,
        DemoDriver::Cleanroom => cleanroom::run(config).await,
        DemoDriver::NeoForge => neoforge::run(config).await,
    }
}

pub(super) async fn ensure_instance(
    config: &DemoConfig,
) -> Result<Storage<BaseInstanceLayout, Storage<BaseRootLayout>>> {
    let storage = Storage::new(config.storage_root.clone(), BaseRootLayout);
    storage
        .ensure_instance(config.instance_name.clone(), BaseInstanceLayout)
        .await
}

pub(super) fn build_launch_config(config: &DemoConfig) -> VanillaLaunchConfig {
    let mut launch_config = VanillaLaunchConfig::new();
    launch_config.runtime_major_version = config.runtime_major_version;
    launch_config.runtime_executable_path = config.runtime_executable_path.clone();
    launch_config
}

pub(super) fn offline_authorizer() -> OfflineAuthorizer {
    OfflineAuthorizer {
        username: "Player".to_owned(),
    }
}

pub(super) fn require_loader_version(config: &DemoConfig, driver_label: &str) -> Result<String> {
    config
        .loader_version
        .clone()
        .with_context(|| format!("{driver_label} demo requires a loader version"))
}

pub(super) async fn prepare_loader_demo(
    config: &DemoConfig,
    driver_label: &str,
) -> Result<(
    String,
    Storage<BaseInstanceLayout, Storage<BaseRootLayout>>,
    VanillaLaunchConfig,
)> {
    let loader_version = require_loader_version(config, driver_label)?;
    let instance = ensure_instance(config).await?;
    let launch_config = build_launch_config(config);

    Ok((loader_version, instance, launch_config))
}

pub(super) async fn run_profiled_version_json_demo<F>(
    config: DemoConfig,
    driver_label: &str,
    driver: &ProfiledVersionJsonDriver<F>,
) -> Result<()>
where
    F: ProfiledVersionJsonFamily,
{
    let (loader_version, instance, launch_config) =
        prepare_loader_demo(&config, driver_label).await?;
    let (prepared, prepare_elapsed) = time_operation(driver.prepare(
        &instance,
        config.game_version.clone(),
        loader_version.clone(),
    ))
    .await?;
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

pub(super) async fn run_installer_family_demo<F>(
    config: DemoConfig,
    driver_label: &str,
    driver: &InstallerFamilyDriver<F>,
) -> Result<()>
where
    F: InstallerFamilyDriverSpec,
{
    let (loader_version, instance, launch_config) =
        prepare_loader_demo(&config, driver_label).await?;
    let (prepared, prepare_elapsed) = time_operation(driver.prepare_with_config(
        &instance,
        config.game_version.clone(),
        loader_version.clone(),
        &launch_config,
    ))
    .await?;
    let (runtime, command) = driver
        .build_launch_command(offline_authorizer(), &prepared, &launch_config)
        .await?;

    finalize_launch(
        &config,
        Some(loader_version.as_str()),
        prepare_elapsed.as_millis(),
        &prepared.install_status,
        &prepared.launch_version.resolved_version.version,
        runtime,
        command,
    )
    .await
}

pub(super) async fn time_operation<T, Fut>(operation: Fut) -> Result<(T, Duration)>
where
    Fut: Future<Output = Result<T>>,
{
    let started_at = std::time::Instant::now();
    let result = operation.await?;
    Ok((result, started_at.elapsed()))
}

pub(super) async fn finalize_launch<L, VL>(
    config: &DemoConfig,
    loader_version: Option<&str>,
    prepared_ms: u128,
    install_status: &dyn Debug,
    version: &Storage<VL, Storage<L>>,
    runtime: Distribution,
    command: LaunchCommand,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let diagnostics = collect_version_diagnostics(version)?;
    let runtime_executable = runtime.executable().to_path_buf();

    print_launch_diagnostics(&LaunchDiagnostics {
        driver_name: config.driver.as_str(),
        loader_version,
        instance_name: &config.instance_name,
        game_version: &config.game_version,
        prepared_ms,
        install_status,
        runtime_executable: runtime_executable.as_path(),
        diagnostics: &diagnostics,
        command: &command,
    });

    let exit_status = run_logged_process(command).await?;
    print_summary(&LaunchSummary {
        driver_name: config.driver.as_str(),
        game_version: &config.game_version,
        loader_version,
        runtime_executable: runtime_executable.as_path(),
        version_root: diagnostics.version_root.as_path(),
        install_status,
        prepared_ms,
        exit_status,
    });

    Ok(())
}
