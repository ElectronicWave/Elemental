use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use elemental::{
    core::minecraft::MinecraftVersionId,
    driver::loader_version::LoaderVersionId,
    launcher::{Instance, LaunchOptions, Launcher},
};
use inquire::Select;

use crate::{
    config::{LaunchpadCommand, LaunchpadConfig, build_spec},
    defaults::{DriverDefaults, driver_defaults},
};

use super::{
    choices::{InstanceChoice, prompt_or_back},
    runtime::{self, RuntimeSelection},
};

pub(super) async fn prompt(storage_root: PathBuf) -> Result<Option<LaunchpadCommand>> {
    let launcher = Launcher::builder()
        .storage_root(storage_root.clone())
        .build();
    let instances = launcher.inspect_instances().await?;
    if instances.is_empty() {
        bail!(
            "No installed instances found under {}",
            storage_root.display()
        );
    }

    let Some(mut selected_instance) = prompt_instance(&instances, None)? else {
        return Ok(None);
    };
    let mut runtime_choice = None;

    loop {
        let defaults = instance_defaults(&selected_instance)?;
        let (game_version, loader_version) = instance_versions(&selected_instance, defaults);
        let Some(runtime) =
            runtime::prompt(defaults.runtime_major_version, &mut runtime_choice).await?
        else {
            let Some(next_instance) = prompt_instance(&instances, Some(&selected_instance))? else {
                return Ok(None);
            };
            selected_instance = next_instance;
            continue;
        };

        return Ok(Some(LaunchpadCommand::Launch(Box::new(launch_config(
            storage_root,
            &selected_instance,
            game_version,
            loader_version,
            runtime,
        )?))));
    }
}

fn instance_defaults(instance: &Instance) -> Result<DriverDefaults> {
    driver_defaults(instance.driver.driver).with_context(|| {
        format!(
            "{} launchpad defaults are missing",
            instance.driver.driver.name
        )
    })
}

fn instance_versions(
    instance: &Instance,
    defaults: DriverDefaults,
) -> (MinecraftVersionId, Option<LoaderVersionId>) {
    let game_version = instance
        .driver
        .game_version
        .as_ref()
        .cloned()
        .unwrap_or_else(|| MinecraftVersionId::from(defaults.game_version));
    let loader_version = instance
        .driver
        .driver_version
        .as_deref()
        .map(LoaderVersionId::from)
        .or_else(|| defaults.loader_version.map(LoaderVersionId::from));

    (game_version, loader_version)
}

fn launch_config(
    storage_root: PathBuf,
    instance: &Instance,
    game_version: MinecraftVersionId,
    loader_version: Option<LoaderVersionId>,
    runtime: RuntimeSelection,
) -> Result<LaunchpadConfig> {
    let spec = build_spec(instance.driver.driver, game_version, loader_version)?;
    let options = LaunchOptions {
        runtime_major_version: runtime.major_version,
        runtime_validation: runtime.validation,
        ..LaunchOptions::default()
    };
    Ok(LaunchpadConfig {
        spec,
        options,
        runtime_paths: Vec::new(),
        storage_root,
        instance_name: instance.instance_name.clone(),
        local_only: true,
    })
}

fn prompt_instance(instances: &[Instance], current: Option<&Instance>) -> Result<Option<Instance>> {
    let starting_cursor = current
        .and_then(|current| {
            instances
                .iter()
                .position(|instance| instance.instance_name == current.instance_name)
        })
        .unwrap_or(0);
    let result = Select::new(
        "Choose an installed instance",
        instances
            .iter()
            .cloned()
            .map(|instance| InstanceChoice { instance })
            .collect(),
    )
    .with_starting_cursor(starting_cursor)
    .with_help_message("Registered instances only. Esc goes back")
    .prompt();

    prompt_or_back(result).map(|choice| choice.map(|choice| choice.instance))
}
