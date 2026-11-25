use tokio::process::Child;

use super::{command::LaunchCommand, error::Result};

pub fn spawn_command(command: LaunchCommand) -> Result<Child> {
    command.spawn()
}
