use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use elemental::core::{minecraft::MinecraftVersionId, runtime::RuntimeValidationMode};
use elemental::driver::{
    descriptors::{
        BABRIC_DRIVER, CLEANROOM_DRIVER, FABRIC_DRIVER, FORGE_DRIVER, LEGACY_FABRIC_DRIVER,
        LITELOADER_DRIVER, NEOFORGE_DRIVER, OUTLANDS_DRIVER, QUILT_DRIVER, RIFT_DRIVER,
        VANILLA_DRIVER,
    },
    driver::DriverDescriptor,
    loader_version::LoaderVersionId,
};

use crate::config::{DemoCommand, DemoConfig};

const CLEANROOM_DEMO_RUNTIME_MAJOR_VERSION: usize = 25;
const OUTLANDS_DEMO_RUNTIME_MAJOR_VERSION: usize = 25;

#[derive(Debug, Parser)]
#[command(name = "demo", about = "Elemental demo CLI")]
#[command(subcommand_precedence_over_arg = true)]
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    storage_root: Option<PathBuf>,
    #[arg(long, global = true, help = "Use only locally prepared data")]
    local_only: bool,
    #[command(flatten)]
    launch: LaunchArgs,
    #[command(subcommand)]
    command: Option<DemoCliCommand>,
}

#[derive(Debug, Subcommand)]
enum DemoCliCommand {
    #[command(name = "list", about = "List local prepared instances")]
    List,
}

#[derive(Debug, Args)]
struct LaunchArgs {
    /// Driver id: vanilla, fabric, legacyfabric, babric, quilt, liteloader,
    /// rift, forge, cleanroom, outlands, or neoforge. Defaults to fabric.
    #[arg(value_parser = parse_driver)]
    driver: Option<DriverDescriptor>,
    /// Minecraft version. Uses the driver default when omitted.
    game_version: Option<String>,
    /// Loader version. Not accepted by vanilla.
    loader_version: Option<String>,
    /// Prepared instance name. Uses the driver default when omitted.
    #[arg(long, value_name = "NAME")]
    instance_name: Option<String>,
    #[arg(long)]
    runtime_major_version: Option<usize>,
    #[arg(long, value_enum, default_value_t = RuntimeValidationArg::Strict)]
    runtime_validation: RuntimeValidationArg,
    #[arg(long = "runtime-path", value_name = "PATH")]
    runtime_paths: Vec<PathBuf>,
    #[arg(long = "runtime-executable", value_name = "PATH")]
    runtime_executable_path: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum, Default)]
enum RuntimeValidationArg {
    #[default]
    Strict,
    Disabled,
}

impl RuntimeValidationArg {
    fn into_runtime_validation_mode(self) -> RuntimeValidationMode {
        match self {
            Self::Strict => RuntimeValidationMode::Strict,
            Self::Disabled => RuntimeValidationMode::Disabled,
        }
    }
}

impl Cli {
    pub fn into_demo_command(self) -> Result<DemoCommand> {
        let storage_root = self
            .storage_root
            .unwrap_or_else(|| PathBuf::from(".minecraft"));

        if matches!(self.command, Some(DemoCliCommand::List)) {
            return Ok(DemoCommand::ListInstances { storage_root });
        }

        Ok(DemoCommand::Launch(build_launch_config(
            storage_root,
            self.local_only,
            self.launch,
        )?))
    }
}

fn parse_driver(value: &str) -> std::result::Result<DriverDescriptor, String> {
    DriverDescriptor::try_from(value).map_err(|error| error.to_string())
}

struct LaunchInput {
    driver: DriverDescriptor,
    local_only: bool,
    storage_root: PathBuf,
    runtime_major_version: Option<usize>,
    runtime_validation: RuntimeValidationMode,
    runtime_paths: Vec<PathBuf>,
    runtime_executable_path: Option<PathBuf>,
    game_version: Option<MinecraftVersionId>,
    loader_version: Option<LoaderVersionId>,
    instance_name: Option<String>,
}

struct LoaderDefaults {
    game_version: &'static str,
    loader_version: &'static str,
    instance_prefix: &'static str,
    runtime_major_version: Option<usize>,
}

fn build_launch_config(
    storage_root: PathBuf,
    local_only: bool,
    arguments: LaunchArgs,
) -> Result<DemoConfig> {
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
        driver: driver.unwrap_or(FABRIC_DRIVER),
        local_only,
        storage_root,
        runtime_major_version,
        runtime_validation: runtime_validation.into_runtime_validation_mode(),
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

fn build_vanilla_config(input: LaunchInput) -> Result<DemoConfig> {
    if input.loader_version.is_some() {
        bail!("vanilla does not accept a loader version");
    }

    let game_version = input
        .game_version
        .unwrap_or_else(|| MinecraftVersionId::from("1.20.1"));
    let instance_name = input
        .instance_name
        .unwrap_or_else(|| format!("MyVanilla-{game_version}"));

    Ok(DemoConfig {
        driver: input.driver,
        local_only: input.local_only,
        storage_root: input.storage_root,
        instance_name,
        game_version,
        loader_version: None,
        runtime_major_version: input.runtime_major_version,
        runtime_validation: input.runtime_validation,
        runtime_paths: input.runtime_paths,
        runtime_executable_path: input.runtime_executable_path,
    })
}

fn build_loader_config(input: LaunchInput) -> Result<DemoConfig> {
    let defaults = loader_defaults(input.driver)
        .with_context(|| format!("{} is not a loader driver", input.driver.name))?;
    let game_version = input
        .game_version
        .unwrap_or_else(|| MinecraftVersionId::from(defaults.game_version));
    let loader_version = input
        .loader_version
        .unwrap_or_else(|| LoaderVersionId::from(defaults.loader_version));
    let instance_name = input
        .instance_name
        .unwrap_or_else(|| format!("{}-{game_version}", defaults.instance_prefix));

    Ok(DemoConfig {
        driver: input.driver,
        local_only: input.local_only,
        storage_root: input.storage_root,
        instance_name,
        game_version,
        loader_version: Some(loader_version),
        runtime_major_version: input
            .runtime_major_version
            .or(defaults.runtime_major_version),
        runtime_validation: input.runtime_validation,
        runtime_paths: input.runtime_paths,
        runtime_executable_path: input.runtime_executable_path,
    })
}

fn loader_defaults(driver: DriverDescriptor) -> Option<LoaderDefaults> {
    match driver {
        FABRIC_DRIVER => Some(LoaderDefaults {
            game_version: "1.20.1",
            loader_version: "0.16.10",
            instance_prefix: "MyFabric",
            runtime_major_version: None,
        }),
        LEGACY_FABRIC_DRIVER => Some(LoaderDefaults {
            game_version: "1.20.1",
            loader_version: "0.16.10",
            instance_prefix: "MyLegacyFabric",
            runtime_major_version: None,
        }),
        BABRIC_DRIVER => Some(LoaderDefaults {
            game_version: "1.20.1",
            loader_version: "0.16.10",
            instance_prefix: "MyBabric",
            runtime_major_version: None,
        }),
        QUILT_DRIVER => Some(LoaderDefaults {
            game_version: "1.20.1",
            loader_version: "0.24.0",
            instance_prefix: "MyQuilt",
            runtime_major_version: None,
        }),
        LITELOADER_DRIVER => Some(LoaderDefaults {
            game_version: "1.7.10",
            loader_version: "1.7.10_04",
            instance_prefix: "MyLiteLoader",
            runtime_major_version: None,
        }),
        RIFT_DRIVER => Some(LoaderDefaults {
            game_version: "1.13.2",
            loader_version: "1.0.4-106",
            instance_prefix: "MyRift",
            runtime_major_version: None,
        }),
        FORGE_DRIVER => Some(LoaderDefaults {
            game_version: "1.20.1",
            loader_version: "47.3.1",
            instance_prefix: "MyForge",
            runtime_major_version: None,
        }),
        CLEANROOM_DRIVER => Some(LoaderDefaults {
            game_version: "1.12.2",
            loader_version: "0.5.8-alpha",
            instance_prefix: "MyCleanroom",
            runtime_major_version: Some(CLEANROOM_DEMO_RUNTIME_MAJOR_VERSION),
        }),
        OUTLANDS_DRIVER => Some(LoaderDefaults {
            game_version: "1.12.2",
            loader_version: "2026.11.06",
            instance_prefix: "MyOutlands",
            runtime_major_version: Some(OUTLANDS_DEMO_RUNTIME_MAJOR_VERSION),
        }),
        NEOFORGE_DRIVER => Some(LoaderDefaults {
            game_version: "1.21.1",
            loader_version: "21.1.199",
            instance_prefix: "MyNeoForge",
            runtime_major_version: None,
        }),
        _ => None,
    }
}
