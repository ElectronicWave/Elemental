use std::time::Duration;

use anyhow::{Context, Result};
use elemental::{
    core::{
        auth::{authorizer::Authorizer, authorizers::offline::OfflineAuthorizer},
        launcher::command::LaunchCommand,
        runtime::distribution::Distribution,
        storage::Storage,
    },
    driver::descriptors::VANILLA_DRIVER,
    driver::families::version_json::{
        BaseInstanceLayout, BaseRootLayout, VersionJsonGameStorageExt,
    },
    launcher::{DriverSpec, LaunchOptions, Launcher},
};

use crate::config::{DemoCommand, DemoConfig};
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
        let session = launcher
            .downloader()
            .session(format!("prepare {request_instance_name}"))
            .open()
            .await?;
        let started_at = std::time::Instant::now();
        let result = launcher
            .prepare_with_session(&session, request_instance_name, request_driver)
            .await;
        let report = session.finish().await?;
        let prepared = result?;
        report.success()?;
        (prepared, started_at.elapsed())
    };

    let credential = offline_authorizer().authorize().await?;
    let command = launcher
        .build_launch_command(&prepared, &credential, &launch_options)
        .await?;

    finalize_launch(
        &config,
        prepare_elapsed.as_millis(),
        command.runtime.clone(),
        command,
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
    if config.driver == VANILLA_DRIVER {
        return Ok(DriverSpec::vanilla(config.game_version.clone()));
    }

    Ok(DriverSpec::any(
        config.driver,
        config.game_version.clone(),
        require_loader_version(config)?,
    )?)
}

fn require_loader_version(
    config: &DemoConfig,
) -> Result<elemental::driver::loader_version::LoaderVersionId> {
    config
        .loader_version
        .clone()
        .with_context(|| format!("{} demo requires a loader version", config.driver.name))
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

pub(super) async fn time_operation<T, E, Fut>(operation: Fut) -> Result<(T, Duration)>
where
    Fut: Future<Output = std::result::Result<T, E>>,
    E: Into<anyhow::Error>,
{
    let started_at = std::time::Instant::now();
    let result = operation.await.map_err(Into::into)?;
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
        driver_name: config.driver.name,
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
        driver_name: config.driver.name,
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
