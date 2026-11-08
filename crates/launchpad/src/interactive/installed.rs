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

struct InstalledSelection {
    instance: Instance,
    defaults: DriverDefaults,
    runtime: RuntimeSelection,
}

impl InstalledSelection {
    fn into_config(self, storage_root: PathBuf) -> Result<LaunchpadConfig> {
        let game_version = self
            .instance
            .driver
            .game_version
            .unwrap_or_else(|| MinecraftVersionId::from(self.defaults.game_version));
        let loader_version = self
            .instance
            .driver
            .driver_version
            .as_deref()
            .map(LoaderVersionId::from)
            .or_else(|| self.defaults.loader_version.map(LoaderVersionId::from));
        let spec = build_spec(self.instance.driver.driver, game_version, loader_version)?;
        let options = LaunchOptions {
            runtime_major_version: self.runtime.major_version,
            runtime_validation: self.runtime.validation,
            ..LaunchOptions::default()
        };

        Ok(LaunchpadConfig {
            spec,
            options,
            runtime_paths: Vec::new(),
            storage_root,
            instance_name: self.instance.instance_name,
            local_only: true,
        })
    }
}

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

    let Some(selection) = select_instance(&instances).await? else {
        return Ok(None);
    };
    let config = selection.into_config(storage_root)?;

    Ok(Some(LaunchpadCommand::Launch(Box::new(config))))
}

async fn select_instance(instances: &[Instance]) -> Result<Option<InstalledSelection>> {
    let Some(mut instance) = prompt_instance(instances, None)? else {
        return Ok(None);
    };
    let mut runtime_choice = None;

    loop {
        let defaults = instance_defaults(&instance)?;
        let runtime = runtime::prompt(defaults.runtime_major_version, &mut runtime_choice).await?;
        if let Some(runtime) = runtime {
            return Ok(Some(InstalledSelection {
                instance,
                defaults,
                runtime,
            }));
        }

        let Some(next_instance) = prompt_instance(instances, Some(&instance))? else {
            return Ok(None);
        };
        instance = next_instance;
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
