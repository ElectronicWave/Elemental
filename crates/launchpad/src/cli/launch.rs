use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use elemental::{
    core::minecraft::MinecraftVersionId,
    driver::{
        descriptors::VANILLA_DRIVER, driver::DriverDescriptor, loader_version::LoaderVersionId,
    },
    launcher::{DriverSpec, LaunchOptions},
};

use crate::{
    config::{LaunchpadConfig, build_spec},
    defaults::{default_instance_name, driver_defaults},
};

use super::args::{LaunchArgs, RuntimeValidationArg, default_driver};

struct LaunchInput {
    driver: DriverDescriptor,
    local_only: bool,
    storage_root: PathBuf,
    runtime_major_version: Option<usize>,
    runtime_validation: RuntimeValidationArg,
    runtime_paths: Vec<PathBuf>,
    runtime_executable_path: Option<PathBuf>,
    game_version: Option<MinecraftVersionId>,
    loader_version: Option<LoaderVersionId>,
    instance_name: Option<String>,
}

impl LaunchInput {
    fn into_config(self, spec: DriverSpec, instance_name: String) -> LaunchpadConfig {
        let options = LaunchOptions {
            runtime_major_version: self.runtime_major_version,
            runtime_executable_path: self.runtime_executable_path,
            runtime_validation: self.runtime_validation.into_runtime_validation_mode(),
            ..LaunchOptions::default()
        };
        LaunchpadConfig {
            spec,
            options,
            runtime_paths: self.runtime_paths,
            storage_root: self.storage_root,
            instance_name,
            local_only: self.local_only,
        }
    }
}

pub(super) fn build(
    storage_root: PathBuf,
    local_only: bool,
    arguments: LaunchArgs,
) -> Result<LaunchpadConfig> {
    let LaunchArgs {
        driver,
        game_version,
        loader_version,
        instance_name,
        runtime_major_version,
        runtime_validation,
        runtime_paths,
        runtime_executable_path,
    } = arguments;
    let input = LaunchInput {
        driver: driver.unwrap_or_else(default_driver),
        local_only,
        storage_root,
        runtime_major_version,
        runtime_validation: runtime_validation.unwrap_or_default(),
        runtime_paths,
        runtime_executable_path,
        game_version: game_version.map(MinecraftVersionId::from),
        loader_version: loader_version.map(LoaderVersionId::from),
        instance_name,
    };

    if input.driver == VANILLA_DRIVER {
        return build_vanilla_config(input);
    }

    build_loader_config(input)
}

fn build_vanilla_config(input: LaunchInput) -> Result<LaunchpadConfig> {
    if input.loader_version.is_some() {
        bail!("vanilla does not accept a loader version");
    }

    let defaults = driver_defaults(input.driver)
        .with_context(|| format!("{} launchpad defaults are missing", input.driver.name))?;
    let game_version = input
        .game_version
        .clone()
        .unwrap_or_else(|| MinecraftVersionId::from(defaults.game_version));
    let instance_name = input
        .instance_name
        .clone()
        .unwrap_or_else(|| default_instance_name(defaults, game_version.as_str()));
    let spec = build_spec(input.driver, game_version, None)?;

    Ok(input.into_config(spec, instance_name))
}

fn build_loader_config(input: LaunchInput) -> Result<LaunchpadConfig> {
    let defaults = driver_defaults(input.driver)
        .with_context(|| format!("{} launchpad defaults are missing", input.driver.name))?;
    let game_version = input
        .game_version
        .clone()
        .unwrap_or_else(|| MinecraftVersionId::from(defaults.game_version));
    let default_loader_version = defaults
        .loader_version
        .with_context(|| format!("{} launchpad requires a loader version", input.driver.name))?;
    let loader_version = input
        .loader_version
        .clone()
        .unwrap_or_else(|| LoaderVersionId::from(default_loader_version));
    let instance_name = input
        .instance_name
        .clone()
        .unwrap_or_else(|| default_instance_name(defaults, game_version.as_str()));
    let mut input = input;
    input.runtime_major_version = input
        .runtime_major_version
        .or(defaults.runtime_major_version);
    let spec = build_spec(input.driver, game_version, Some(loader_version))?;

    Ok(input.into_config(spec, instance_name))
}
