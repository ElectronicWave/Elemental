use std::time::Duration;

use anyhow::{Context, Result};
use elemental::{
    core::{
        auth::authorizers::offline::OfflineAuthorizer, launcher::command::LaunchCommand,
        runtime::distribution::Distribution, storage::Storage,
    },
    driver::families::version_json::{
        BaseInstanceLayout, BaseRootLayout, VersionJsonGameStorageExt,
    },
    launcher::{
        DriverSpec, LaunchOptions, Launcher, LoaderSpec, PrepareInstanceRequest, VanillaSpec,
    },
};

use crate::config::{DemoCommand, DemoConfig, DemoDriver};
use crate::diagnostics::{
    LaunchDiagnostics, LaunchSummary, collect_version_diagnostics, print_launch_diagnostics,
    print_summary, run_logged_process,
};

pub async fn run(command: DemoCommand) -> Result<()> {
    match command {
        DemoCommand::Launch(config) => run_launch(config).await,
        DemoCommand::ListInstances { storage_root } => run_list_instances(storage_root).await,
    }
}

async fn run_launch(config: DemoConfig) -> Result<()> {
    let launcher = Launcher::builder()
        .storage_root(config.storage_root.clone())
        .build();
    let launch_options: LaunchOptions = build_launch_config(&config);
    let driver_spec = to_driver_spec(&config)?;

    let request_instance_name = config.instance_name.clone();
    let request_driver = driver_spec.clone();

    let (prepared, prepare_elapsed) = if config.local_only {
        time_operation(
            launcher.load_instance(
                launcher
                    .inspect_instance(request_instance_name)
                    .await?
                    .context("Can't find instance")?,
            ),
        )
        .await?
    } else {
        time_operation(launcher.prepare_instance(PrepareInstanceRequest {
            instance_name: request_instance_name,
            driver: request_driver,
        }))
        .await?
    };

    let command_result = launcher
        .build_launch_command(&prepared, offline_authorizer(), &launch_options)
        .await?;

    finalize_launch(
        &config,
        prepare_elapsed.as_millis(),
        command_result.runtime,
        command_result.command,
    )
    .await
}

async fn run_list_instances(storage_root: std::path::PathBuf) -> Result<()> {
    let launcher = Launcher::builder().storage_root(storage_root).build();
    let instances = launcher.inspect_instances().await?;

    if instances.is_empty() {
        println!("No local instances found.");
        return Ok(());
    }

    println!("Found {} local instance(s):", instances.len());
    for instance in instances {
        println!(
            "- {} | driver={} | root={}",
            instance.instance_name,
            instance.driver.driver.id,
            instance.instance_root.display()
        );
    }

    Ok(())
}

fn to_driver_spec(config: &DemoConfig) -> Result<DriverSpec> {
    let game_version = config.game_version.clone();

    Ok(match config.driver {
        DemoDriver::Vanilla => DriverSpec::Vanilla(VanillaSpec { game_version }),
        DemoDriver::Fabric => DriverSpec::Fabric(LoaderSpec {
            game_version,
            loader_version: require_loader_version(config, "fabric")?,
        }),
        DemoDriver::LegacyFabric => DriverSpec::LegacyFabric(LoaderSpec {
            game_version,
            loader_version: require_loader_version(config, "legacyfabric")?,
        }),
        DemoDriver::Babric => DriverSpec::Babric(LoaderSpec {
            game_version,
            loader_version: require_loader_version(config, "babric")?,
        }),
        DemoDriver::Quilt => DriverSpec::Quilt(LoaderSpec {
            game_version,
            loader_version: require_loader_version(config, "quilt")?,
        }),
        DemoDriver::LiteLoader => DriverSpec::LiteLoader(LoaderSpec {
            game_version,
            loader_version: require_loader_version(config, "liteloader")?,
        }),
        DemoDriver::Rift => DriverSpec::Rift(LoaderSpec {
            game_version,
            loader_version: require_loader_version(config, "rift")?,
        }),
        DemoDriver::Forge => DriverSpec::Forge(LoaderSpec {
            game_version,
            loader_version: require_loader_version(config, "forge")?,
        }),
        DemoDriver::Cleanroom => DriverSpec::Cleanroom(LoaderSpec {
            game_version,
            loader_version: require_loader_version(config, "cleanroom")?,
        }),
        DemoDriver::NeoForge => DriverSpec::NeoForge(LoaderSpec {
            game_version,
            loader_version: require_loader_version(config, "neoforge")?,
        }),
    })
}

fn require_loader_version(
    config: &DemoConfig,
    driver_label: &str,
) -> Result<elemental::driver::loader_version::LoaderVersionId> {
    config
        .loader_version
        .clone()
        .with_context(|| format!("{driver_label} demo requires a loader version"))
}

pub(super) fn build_launch_config(config: &DemoConfig) -> LaunchOptions {
    let mut launch_config = LaunchOptions::new();
    launch_config.runtime_major_version = config.runtime_major_version;
    launch_config.runtime_executable_path = config.runtime_executable_path.clone();
    launch_config.runtime_validation = config.runtime_validation;
    launch_config
}

pub(super) fn offline_authorizer() -> OfflineAuthorizer {
    OfflineAuthorizer {
        username: "Player".to_owned(),
    }
}

pub(super) async fn time_operation<T, Fut>(operation: Fut) -> Result<(T, Duration)>
where
    Fut: Future<Output = Result<T>>,
{
    let started_at = std::time::Instant::now();
    let result = operation.await?;
    Ok((result, started_at.elapsed()))
}

use std::future::Future;

pub(super) async fn finalize_launch(
    config: &DemoConfig,
    prepared_ms: u128,
    runtime: Distribution,
    command: LaunchCommand,
) -> Result<()> {
    let storage = Storage::new(config.storage_root.clone(), BaseRootLayout);
    let version = storage.instance(config.instance_name.clone(), BaseInstanceLayout)?;
    let diagnostics = collect_version_diagnostics(&version)?;
    let runtime_executable = runtime.executable().to_path_buf();
    let loader_version = config
        .loader_version
        .as_ref()
        .map(|version| version.as_str());
    let install_status = if config.local_only {
        "LoadedPrepared(via facade)"
    } else {
        "Prepared(via facade)"
    };

    print_launch_diagnostics(&LaunchDiagnostics {
        driver_name: config.driver.as_str(),
        loader_version,
        instance_name: &config.instance_name,
        game_version: config.game_version.as_str(),
        prepared_ms,
        install_status: &install_status,
        runtime_executable: runtime_executable.as_path(),
        diagnostics: &diagnostics,
        command: &command,
    });

    let exit_status = run_logged_process(command).await?;
    print_summary(&LaunchSummary {
        driver_name: config.driver.as_str(),
        game_version: config.game_version.as_str(),
        loader_version,
        runtime_executable: runtime_executable.as_path(),
        version_root: diagnostics.version_root.as_path(),
        install_status: &install_status,
        prepared_ms,
        exit_status,
    });

    Ok(())
}
