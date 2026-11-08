use std::path::PathBuf;

use anyhow::{Context, Result};
use elemental::{
    core::minecraft::MinecraftVersionId,
    driver::{driver::DriverDescriptor, loader_version::LoaderVersionId},
    launcher::{LaunchOptions, RuntimeOptions},
};

use crate::{
    config::{LaunchpadConfig, build_spec},
    defaults::{DriverDefaults, default_instance_name, driver_defaults},
};

use super::args::{LaunchArgs, RuntimeValidationArg, default_driver};

struct LaunchInput {
    driver: DriverDescriptor,
    local_only: bool,
    storage_root: PathBuf,
    runtime_major_version: Option<usize>,
    runtime_validation: RuntimeValidationArg,
    runtime_paths: Vec<PathBuf>,
    game_version: Option<MinecraftVersionId>,
    loader_version: Option<LoaderVersionId>,
    instance_name: Option<String>,
}

impl LaunchInput {
    fn into_config(self, defaults: DriverDefaults) -> Result<LaunchpadConfig> {
        let game_version = self
            .game_version
            .unwrap_or_else(|| MinecraftVersionId::from(defaults.game_version));
        let instance_name = self
            .instance_name
            .unwrap_or_else(|| default_instance_name(defaults, game_version.as_str()));
        let loader_version = self
            .loader_version
            .or_else(|| defaults.loader_version.map(LoaderVersionId::from));
        let spec = build_spec(self.driver, game_version, loader_version)?;
        let runtime = RuntimeOptions {
            runtime_major_version: self
                .runtime_major_version
                .or(defaults.runtime_major_version),
            runtime_validation: self.runtime_validation.into_runtime_validation_mode(),
            ..RuntimeOptions::default()
        };
        let options = LaunchOptions::default();

        Ok(LaunchpadConfig {
            spec,
            runtime,
            options,
            runtime_paths: self.runtime_paths,
            storage_root: self.storage_root,
            instance_name,
            local_only: self.local_only,
        })
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
    } = arguments;
    let input = LaunchInput {
        driver: driver.unwrap_or_else(default_driver),
        local_only,
        storage_root,
        runtime_major_version,
        runtime_validation: runtime_validation.unwrap_or_default(),
        runtime_paths,
        game_version: game_version.map(MinecraftVersionId::from),
        loader_version: loader_version.map(LoaderVersionId::from),
        instance_name,
    };
    let defaults = driver_defaults(input.driver)
        .with_context(|| format!("{} launchpad defaults are missing", input.driver.name))?;

    input.into_config(defaults)
}
