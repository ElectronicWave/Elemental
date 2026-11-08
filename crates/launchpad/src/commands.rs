use std::time::Duration;

use anyhow::{Context, Result};
use elemental::{
    core::{
        auth::{authorizer::Authorizer, authorizers::offline::OfflineAuthorizer},
        launcher::command::LaunchCommand,
    },
    launcher::{Launcher, PreparedInstance},
};

use crate::config::{LaunchpadCommand, LaunchpadConfig};
use crate::diagnostics::{
    LaunchDiagnostics, LaunchSummary, collect_version_diagnostics, print_launch_diagnostics,
    print_summary, run_logged_process,
};
use crate::progress;

pub async fn run(command: LaunchpadCommand) -> Result<()> {
    match command {
        LaunchpadCommand::Launch(config) => run_launch(*config).await,
        LaunchpadCommand::ListInstances { storage_root } => run_list_instances(storage_root).await,
        LaunchpadCommand::Exit => Ok(()),
    }
}

async fn run_launch(config: LaunchpadConfig) -> Result<()> {
    let launcher = Launcher::builder()
        .storage_root(config.storage_root.clone())
        .build();

    let request_instance_name = config.instance_name.clone();

    let (prepared, prepare_elapsed) = if config.local_only {
        progress::with_spinner(
            "Loading prepared instance",
            time_operation(
                launcher.load_instance(
                    launcher
                        .inspect_instance(request_instance_name)
                        .await?
                        .context("Can't find instance")?,
                ),
            ),
        )
        .await?
    } else {
        let preparation = launcher
            .begin_prepare(request_instance_name, config.spec.clone())
            .await?;
        let _download_progress = progress::download(preparation.session()).await?;
        let started_at = std::time::Instant::now();
        let prepared = preparation.prepare().await?;
        (prepared, started_at.elapsed())
    };

    let credential = offline_authorizer().authorize().await?;
    let command = launcher
        .build_launch_command(&prepared, &credential, &config.options)
        .await?;

    finalize_launch(&config, &prepared, prepare_elapsed.as_millis(), command).await
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
    config: &LaunchpadConfig,
    prepared: &PreparedInstance,
    prepared_ms: u128,
    command: LaunchCommand,
) -> Result<()> {
    let diagnostics = collect_version_diagnostics(prepared.version())?;
    let runtime_executable = command.runtime.executable().to_path_buf();
    let loader_version = config.spec.loader_version().map(|version| version.as_str());
    let install_status = if config.local_only {
        "LoadedPrepared(via facade)"
    } else {
        "Prepared(via facade)"
    };

    print_launch_diagnostics(&LaunchDiagnostics {
        driver_name: config.spec.name(),
        loader_version,
        instance_name: &config.instance_name,
        game_version: config.spec.game_version().as_str(),
        prepared_ms,
        install_status: &install_status,
        runtime_executable: runtime_executable.as_path(),
        diagnostics: &diagnostics,
        command: &command,
    });

    let exit_status = run_logged_process(command).await?;
    print_summary(&LaunchSummary {
        driver_name: config.spec.name(),
        game_version: config.spec.game_version().as_str(),
        loader_version,
        runtime_executable: runtime_executable.as_path(),
        version_root: diagnostics.version_root.as_path(),
        install_status: &install_status,
        prepared_ms,
        exit_status,
    });

    Ok(())
}
