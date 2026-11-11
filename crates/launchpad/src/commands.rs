use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use elemental::{
    core::{
        auth::{
            accounts::Account, authorizers::Authorizer, authorizers::offline::OfflineAuthorizer,
        },
        launcher::command::LaunchCommand,
        runtime::provider::{RuntimeProvider, default_providers},
        runtime::providers::custom::new_custom_provider,
    },
    launcher::{Launcher, PreparedInstance},
};

use crate::config::{LaunchpadCommand, LaunchpadConfig};
use crate::diagnostics::{
    LaunchDiagnostics, LaunchSummary, collect_version_diagnostics, print_launch_diagnostics,
    print_summary, run_process,
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
    let launcher = build_launcher(&config)?;

    let (prepared, prepare_elapsed) = prepare_instance(&launcher, &config).await?;

    let account = OfflineAuthorizer {
        username: "Player".to_owned(),
    }
    .authorize()
    .await?;
    let credential = account.credential().await?;
    let command = launcher
        .build_launch_command(&prepared, &credential, &config.runtime, &config.options)
        .await?;

    finalize_launch(
        &launcher,
        &config,
        &prepared,
        prepare_elapsed.as_millis(),
        command,
    )
    .await
}

fn build_launcher(config: &LaunchpadConfig) -> Result<Launcher> {
    let mut providers = Vec::<Arc<dyn RuntimeProvider>>::new();
    if !config.runtime_paths.is_empty() {
        providers.push(Arc::new(new_custom_provider(config.runtime_paths.clone())?));
    }
    providers.extend(default_providers());

    Ok(Launcher::builder()
        .storage_root(config.storage_root.clone())
        .runtime_providers(providers)
        .build())
}

async fn prepare_instance(
    launcher: &Launcher,
    config: &LaunchpadConfig,
) -> Result<(PreparedInstance, Duration)> {
    if config.local_only {
        let instance = launcher
            .inspect_instance(config.instance_name.clone())
            .await?
            .context("Can't find instance")?;
        let started_at = Instant::now();
        let prepared = progress::with_spinner(
            "Loading prepared instance",
            launcher.load_instance(instance),
        )
        .await?;
        Ok((prepared, started_at.elapsed()))
    } else {
        let session = launcher
            .downloader()
            .session(format!("prepare {}", config.instance_name))
            .open()
            .await?;
        let _download_progress = progress::download(&session).await?;
        let started_at = Instant::now();
        let prepared = launcher
            .prepare_with_session(
                session,
                config.instance_name.clone(),
                config.spec.clone(),
                &config.runtime,
            )
            .await?;
        Ok((prepared, started_at.elapsed()))
    }
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

pub(super) async fn finalize_launch(
    launcher: &Launcher,
    config: &LaunchpadConfig,
    prepared: &PreparedInstance,
    prepared_ms: u128,
    command: LaunchCommand,
) -> Result<()> {
    let info = prepared.info()?;
    let diagnostics = collect_version_diagnostics(&info);
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

    let exit_status = run_process(launcher, command).await?;
    print_summary(&LaunchSummary {
        driver_name: config.spec.name(),
        game_version: config.spec.game_version().as_str(),
        loader_version,
        runtime_executable: runtime_executable.as_path(),
        version_root: info.version_root.as_path(),
        install_status: &install_status,
        prepared_ms,
        exit_status,
    });

    Ok(())
}
