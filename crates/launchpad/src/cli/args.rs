use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use elemental::{
    core::runtime::RuntimeValidationMode,
    driver::{descriptors::FABRIC_DRIVER, driver::DriverDescriptor},
};

#[derive(Debug, Parser)]
#[command(name = "launchpad", about = "Elemental launchpad CLI")]
#[command(subcommand_precedence_over_arg = true)]
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    pub(super) storage_root: Option<PathBuf>,
    #[arg(long, global = true, help = "Use only locally prepared data")]
    pub(super) local_only: bool,
    #[command(flatten)]
    pub(super) launch: LaunchArgs,
    #[command(subcommand)]
    pub(super) command: Option<LaunchpadCliCommand>,
}

#[derive(Debug, Subcommand)]
pub(super) enum LaunchpadCliCommand {
    #[command(name = "list", about = "List local prepared instances")]
    List,
}

#[derive(Debug, Args)]
pub(super) struct LaunchArgs {
    /// Driver id: vanilla, fabric, legacyfabric, babric, quilt, liteloader,
    /// rift, forge, cleanroom, outlands, or neoforge. Defaults to fabric.
    #[arg(value_parser = parse_driver)]
    pub(super) driver: Option<DriverDescriptor>,
    /// Minecraft version. Uses the driver default when omitted.
    pub(super) game_version: Option<String>,
    /// Loader version. Not accepted by vanilla.
    pub(super) loader_version: Option<String>,
    /// Prepared instance name. Uses the driver default when omitted.
    #[arg(long, value_name = "NAME")]
    pub(super) instance_name: Option<String>,
    #[arg(long)]
    pub(super) runtime_major_version: Option<usize>,
    #[arg(
        long,
        value_enum,
        help = "Validate the selected Java version or skip the check (default: strict)"
    )]
    pub(super) runtime_validation: Option<RuntimeValidationArg>,
    #[arg(long = "runtime-path", value_name = "PATH")]
    pub(super) runtime_paths: Vec<PathBuf>,
}

impl LaunchArgs {
    pub(super) fn is_empty(&self) -> bool {
        self.driver.is_none()
            && self.game_version.is_none()
            && self.loader_version.is_none()
            && self.instance_name.is_none()
            && self.runtime_major_version.is_none()
            && self.runtime_validation.is_none()
            && self.runtime_paths.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum, Default)]
pub(super) enum RuntimeValidationArg {
    #[default]
    Strict,
    Disabled,
}

impl RuntimeValidationArg {
    pub(super) fn into_runtime_validation_mode(self) -> RuntimeValidationMode {
        match self {
            Self::Strict => RuntimeValidationMode::Strict,
            Self::Disabled => RuntimeValidationMode::Disabled,
        }
    }
}

pub(super) fn default_driver() -> DriverDescriptor {
    FABRIC_DRIVER
}

fn parse_driver(value: &str) -> std::result::Result<DriverDescriptor, String> {
    DriverDescriptor::try_from(value).map_err(|error| error.to_string())
}
