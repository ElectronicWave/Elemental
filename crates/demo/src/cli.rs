use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use elemental::core::{minecraft::MinecraftVersionId, runtime::RuntimeValidationMode};
use elemental::driver::loader_version::LoaderVersionId;

use crate::config::{DemoConfig, DemoDriver};

const CLEANROOM_DEMO_RUNTIME_MAJOR_VERSION: usize = 25;

#[derive(Debug, Parser)]
#[command(name = "demo", about = "Elemental demo CLI")]
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    storage_root: Option<PathBuf>,
    #[arg(long, global = true, help = "Use only locally prepared data")]
    local_only: bool,
    #[command(subcommand)]
    command: Option<DriverCommand>,
}

#[derive(Debug, Subcommand)]
enum DriverCommand {
    Vanilla(VanillaArgs),
    Fabric(LoaderArgs),
    #[command(name = "legacyfabric", alias = "legacy-fabric")]
    LegacyFabric(LoaderArgs),
    Babric(LoaderArgs),
    Quilt(LoaderArgs),
    #[command(name = "liteloader", alias = "lite-loader")]
    LiteLoader(LoaderArgs),
    Rift(LoaderArgs),
    Forge(LoaderArgs),
    Cleanroom(LoaderArgs),
    #[command(name = "neoforge", alias = "neo-forge")]
    NeoForge(LoaderArgs),
}

#[derive(Clone, Debug, Default, Args)]
struct VanillaArgs {
    #[arg(long)]
    runtime_major_version: Option<usize>,
    #[arg(long, value_enum, default_value_t = RuntimeValidationArg::Strict)]
    runtime_validation: RuntimeValidationArg,
    #[arg(long = "runtime-path", value_name = "PATH")]
    runtime_paths: Vec<PathBuf>,
    #[arg(long = "runtime-executable", value_name = "PATH")]
    runtime_executable_path: Option<PathBuf>,
    game_version: Option<String>,
    instance_name: Option<String>,
}

#[derive(Clone, Debug, Default, Args)]
struct LoaderArgs {
    #[arg(long)]
    runtime_major_version: Option<usize>,
    #[arg(long, value_enum, default_value_t = RuntimeValidationArg::Strict)]
    runtime_validation: RuntimeValidationArg,
    #[arg(long = "runtime-path", value_name = "PATH")]
    runtime_paths: Vec<PathBuf>,
    #[arg(long = "runtime-executable", value_name = "PATH")]
    runtime_executable_path: Option<PathBuf>,
    game_version: Option<String>,
    loader_version: Option<String>,
    instance_name: Option<String>,
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
    pub fn into_demo_config(self) -> DemoConfig {
        let storage_root = self
            .storage_root
            .unwrap_or_else(|| PathBuf::from(".minecraft"));
        let local_only = self.local_only;

        match self
            .command
            .unwrap_or(DriverCommand::Fabric(LoaderArgs::default()))
        {
            DriverCommand::Vanilla(arguments) => build_vanilla_config(
                local_only,
                common_config_from_vanilla_args(storage_root, arguments),
            ),
            DriverCommand::Fabric(arguments) => build_loader_config(loader_config_input(
                local_only,
                storage_root,
                DemoDriver::Fabric,
                arguments,
            )),
            DriverCommand::LegacyFabric(arguments) => build_loader_config(loader_config_input(
                local_only,
                storage_root,
                DemoDriver::LegacyFabric,
                arguments,
            )),
            DriverCommand::Babric(arguments) => build_loader_config(loader_config_input(
                local_only,
                storage_root,
                DemoDriver::Babric,
                arguments,
            )),
            DriverCommand::Quilt(arguments) => build_loader_config(loader_config_input(
                local_only,
                storage_root,
                DemoDriver::Quilt,
                arguments,
            )),
            DriverCommand::LiteLoader(arguments) => build_loader_config(loader_config_input(
                local_only,
                storage_root,
                DemoDriver::LiteLoader,
                arguments,
            )),
            DriverCommand::Rift(arguments) => build_loader_config(loader_config_input(
                local_only,
                storage_root,
                DemoDriver::Rift,
                arguments,
            )),
            DriverCommand::Forge(arguments) => build_loader_config(loader_config_input(
                local_only,
                storage_root,
                DemoDriver::Forge,
                arguments,
            )),
            DriverCommand::Cleanroom(arguments) => {
                build_loader_config(cleanroom_config_input(local_only, storage_root, arguments))
            }
            DriverCommand::NeoForge(arguments) => build_loader_config(loader_config_input(
                local_only,
                storage_root,
                DemoDriver::NeoForge,
                arguments,
            )),
        }
    }
}

struct CommonConfigInput {
    storage_root: PathBuf,
    runtime_major_version: Option<usize>,
    runtime_validation: RuntimeValidationMode,
    runtime_paths: Vec<PathBuf>,
    runtime_executable_path: Option<PathBuf>,
    game_version: Option<MinecraftVersionId>,
    instance_name: Option<String>,
}

struct LoaderConfigInput {
    driver: DemoDriver,
    local_only: bool,
    common: CommonConfigInput,
    loader_version: Option<LoaderVersionId>,
}

fn common_config_from_vanilla_args(
    storage_root: PathBuf,
    arguments: VanillaArgs,
) -> CommonConfigInput {
    let VanillaArgs {
        runtime_major_version,
        runtime_validation,
        runtime_paths,
        runtime_executable_path,
        game_version,
        instance_name,
    } = arguments;

    CommonConfigInput {
        storage_root,
        runtime_major_version,
        runtime_validation: runtime_validation.into_runtime_validation_mode(),
        runtime_paths,
        runtime_executable_path,
        game_version: game_version.map(MinecraftVersionId::from),
        instance_name,
    }
}

fn loader_config_input(
    local_only: bool,
    storage_root: PathBuf,
    driver: DemoDriver,
    arguments: LoaderArgs,
) -> LoaderConfigInput {
    let LoaderArgs {
        runtime_major_version,
        runtime_validation,
        runtime_paths,
        runtime_executable_path,
        game_version,
        loader_version,
        instance_name,
    } = arguments;

    LoaderConfigInput {
        driver,
        local_only,
        common: CommonConfigInput {
            storage_root,
            runtime_major_version,
            runtime_validation: runtime_validation.into_runtime_validation_mode(),
            runtime_paths,
            runtime_executable_path,
            game_version: game_version.map(MinecraftVersionId::from),
            instance_name,
        },
        loader_version: loader_version.map(LoaderVersionId::from),
    }
}

fn cleanroom_config_input(
    local_only: bool,
    storage_root: PathBuf,
    arguments: LoaderArgs,
) -> LoaderConfigInput {
    let LoaderArgs {
        runtime_major_version,
        runtime_validation,
        runtime_paths,
        runtime_executable_path,
        game_version,
        loader_version,
        instance_name,
    } = arguments;

    LoaderConfigInput {
        driver: DemoDriver::Cleanroom,
        local_only,
        common: CommonConfigInput {
            storage_root,
            runtime_major_version: runtime_major_version
                .or(Some(CLEANROOM_DEMO_RUNTIME_MAJOR_VERSION)),
            runtime_validation: runtime_validation.into_runtime_validation_mode(),
            runtime_paths,
            runtime_executable_path,
            game_version: game_version.map(MinecraftVersionId::from),
            instance_name,
        },
        loader_version: loader_version.map(LoaderVersionId::from),
    }
}

fn build_vanilla_config(local_only: bool, input: CommonConfigInput) -> DemoConfig {
    let resolved_game_version = input
        .game_version
        .unwrap_or_else(|| MinecraftVersionId::from("1.20.1"));
    let resolved_instance_name = input
        .instance_name
        .unwrap_or_else(|| format!("MyVanilla-{resolved_game_version}"));

    DemoConfig {
        driver: DemoDriver::Vanilla,
        local_only,
        storage_root: input.storage_root,
        instance_name: resolved_instance_name,
        game_version: resolved_game_version,
        loader_version: None,
        runtime_major_version: input.runtime_major_version,
        runtime_validation: input.runtime_validation,
        runtime_paths: input.runtime_paths,
        runtime_executable_path: input.runtime_executable_path,
    }
}

fn build_loader_config(input: LoaderConfigInput) -> DemoConfig {
    let resolved_game_version =
        default_loader_game_version(input.driver, input.common.game_version);
    let resolved_loader_version = default_loader_version(input.driver, input.loader_version);
    let resolved_instance_name = input
        .common
        .instance_name
        .unwrap_or_else(|| default_loader_instance_name(input.driver, &resolved_game_version));

    DemoConfig {
        driver: input.driver,
        local_only: input.local_only,
        storage_root: input.common.storage_root,
        instance_name: resolved_instance_name,
        game_version: resolved_game_version,
        loader_version: Some(resolved_loader_version),
        runtime_major_version: input.common.runtime_major_version,
        runtime_validation: input.common.runtime_validation,
        runtime_paths: input.common.runtime_paths,
        runtime_executable_path: input.common.runtime_executable_path,
    }
}

fn default_loader_game_version(
    driver: DemoDriver,
    game_version: Option<MinecraftVersionId>,
) -> MinecraftVersionId {
    game_version.unwrap_or_else(|| match driver {
        DemoDriver::Fabric => MinecraftVersionId::from("1.20.1"),
        DemoDriver::LegacyFabric => MinecraftVersionId::from("1.20.1"),
        DemoDriver::Babric => MinecraftVersionId::from("1.20.1"),
        DemoDriver::Quilt => MinecraftVersionId::from("1.20.1"),
        DemoDriver::LiteLoader => MinecraftVersionId::from("1.7.10"),
        DemoDriver::Rift => MinecraftVersionId::from("1.13.2"),
        DemoDriver::Forge => MinecraftVersionId::from("1.20.1"),
        DemoDriver::Cleanroom => MinecraftVersionId::from("1.12.2"),
        DemoDriver::NeoForge => MinecraftVersionId::from("1.21.1"),
        DemoDriver::Vanilla => unreachable!("vanilla is handled separately"),
    })
}

fn default_loader_version(
    driver: DemoDriver,
    loader_version: Option<LoaderVersionId>,
) -> LoaderVersionId {
    loader_version.unwrap_or_else(|| match driver {
        DemoDriver::Fabric => LoaderVersionId::from("0.16.10"),
        DemoDriver::LegacyFabric => LoaderVersionId::from("0.16.10"),
        DemoDriver::Babric => LoaderVersionId::from("0.16.10"),
        DemoDriver::Quilt => LoaderVersionId::from("0.24.0"),
        DemoDriver::LiteLoader => LoaderVersionId::from("1.7.10_04"),
        DemoDriver::Rift => LoaderVersionId::from("1.0.4-106"),
        DemoDriver::Forge => LoaderVersionId::from("47.3.1"),
        DemoDriver::Cleanroom => LoaderVersionId::from("0.5.8-alpha"),
        DemoDriver::NeoForge => LoaderVersionId::from("21.1.199"),
        DemoDriver::Vanilla => unreachable!("vanilla is handled separately"),
    })
}

fn default_loader_instance_name(driver: DemoDriver, game_version: &MinecraftVersionId) -> String {
    match driver {
        DemoDriver::Fabric => format!("MyFabric-{game_version}"),
        DemoDriver::LegacyFabric => format!("MyLegacyFabric-{game_version}"),
        DemoDriver::Babric => format!("MyBabric-{game_version}"),
        DemoDriver::Quilt => format!("MyQuilt-{game_version}"),
        DemoDriver::LiteLoader => format!("MyLiteLoader-{game_version}"),
        DemoDriver::Rift => format!("MyRift-{game_version}"),
        DemoDriver::Forge => format!("MyForge-{game_version}"),
        DemoDriver::Cleanroom => format!("MyCleanroom-{game_version}"),
        DemoDriver::NeoForge => format!("MyNeoForge-{game_version}"),
        DemoDriver::Vanilla => unreachable!("vanilla is handled separately"),
    }
}
