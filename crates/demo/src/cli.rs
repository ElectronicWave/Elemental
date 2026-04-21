use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::config::{DemoConfig, DemoDriver};

#[derive(Debug, Parser)]
#[command(name = "demo", about = "Elemental demo CLI")]
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    storage_root: Option<PathBuf>,
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
    Forge(LoaderArgs),
}

#[derive(Clone, Debug, Default, Args)]
struct VanillaArgs {
    game_version: Option<String>,
    instance_name: Option<String>,
}

#[derive(Clone, Debug, Default, Args)]
struct LoaderArgs {
    game_version: Option<String>,
    loader_version: Option<String>,
    instance_name: Option<String>,
}

impl Cli {
    pub fn into_demo_config(self) -> DemoConfig {
        let storage_root = self
            .storage_root
            .unwrap_or_else(|| PathBuf::from(".minecraft"));

        match self
            .command
            .unwrap_or(DriverCommand::Fabric(LoaderArgs::default()))
        {
            DriverCommand::Vanilla(arguments) => build_vanilla_config(
                storage_root,
                arguments.game_version,
                arguments.instance_name,
            ),
            DriverCommand::Fabric(arguments) => build_loader_config(
                DemoDriver::Fabric,
                storage_root,
                arguments.game_version,
                arguments.loader_version,
                arguments.instance_name,
            ),
            DriverCommand::LegacyFabric(arguments) => build_loader_config(
                DemoDriver::LegacyFabric,
                storage_root,
                arguments.game_version,
                arguments.loader_version,
                arguments.instance_name,
            ),
            DriverCommand::Babric(arguments) => build_loader_config(
                DemoDriver::Babric,
                storage_root,
                arguments.game_version,
                arguments.loader_version,
                arguments.instance_name,
            ),
            DriverCommand::Quilt(arguments) => build_loader_config(
                DemoDriver::Quilt,
                storage_root,
                arguments.game_version,
                arguments.loader_version,
                arguments.instance_name,
            ),
            DriverCommand::Forge(arguments) => build_loader_config(
                DemoDriver::Forge,
                storage_root,
                arguments.game_version,
                arguments.loader_version,
                arguments.instance_name,
            ),
        }
    }
}

fn build_vanilla_config(
    storage_root: PathBuf,
    game_version: Option<String>,
    instance_name: Option<String>,
) -> DemoConfig {
    let resolved_game_version = game_version.unwrap_or_else(|| "1.20.1".to_owned());
    let resolved_instance_name =
        instance_name.unwrap_or_else(|| format!("MyVanilla-{resolved_game_version}"));

    DemoConfig {
        driver: DemoDriver::Vanilla,
        storage_root,
        instance_name: resolved_instance_name,
        game_version: resolved_game_version,
        loader_version: None,
    }
}

fn build_loader_config(
    driver: DemoDriver,
    storage_root: PathBuf,
    game_version: Option<String>,
    loader_version: Option<String>,
    instance_name: Option<String>,
) -> DemoConfig {
    let resolved_game_version = default_loader_game_version(driver, game_version);
    let resolved_loader_version = default_loader_version(driver, loader_version);
    let resolved_instance_name = instance_name
        .unwrap_or_else(|| default_loader_instance_name(driver, &resolved_game_version));

    DemoConfig {
        driver,
        storage_root,
        instance_name: resolved_instance_name,
        game_version: resolved_game_version,
        loader_version: Some(resolved_loader_version),
    }
}

fn default_loader_game_version(driver: DemoDriver, game_version: Option<String>) -> String {
    game_version.unwrap_or_else(|| match driver {
        DemoDriver::Fabric => "1.20.1".to_owned(),
        DemoDriver::LegacyFabric => "1.20.1".to_owned(),
        DemoDriver::Babric => "1.20.1".to_owned(),
        DemoDriver::Quilt => "1.20.1".to_owned(),
        DemoDriver::Forge => "1.20.1".to_owned(),
        DemoDriver::Vanilla => unreachable!("vanilla is handled separately"),
    })
}

fn default_loader_version(driver: DemoDriver, loader_version: Option<String>) -> String {
    loader_version.unwrap_or_else(|| match driver {
        DemoDriver::Fabric => "0.16.10".to_owned(),
        DemoDriver::LegacyFabric => "0.16.10".to_owned(),
        DemoDriver::Babric => "0.16.10".to_owned(),
        DemoDriver::Quilt => "0.24.0".to_owned(),
        DemoDriver::Forge => "47.3.1".to_owned(),
        DemoDriver::Vanilla => unreachable!("vanilla is handled separately"),
    })
}

fn default_loader_instance_name(driver: DemoDriver, game_version: &str) -> String {
    match driver {
        DemoDriver::Fabric => format!("MyFabric-{game_version}"),
        DemoDriver::LegacyFabric => format!("MyLegacyFabric-{game_version}"),
        DemoDriver::Babric => format!("MyBabric-{game_version}"),
        DemoDriver::Quilt => format!("MyQuilt-{game_version}"),
        DemoDriver::Forge => format!("MyForge-{game_version}"),
        DemoDriver::Vanilla => unreachable!("vanilla is handled separately"),
    }
}
