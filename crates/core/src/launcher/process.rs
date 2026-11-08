use tokio::process::{Child, Command};

use super::{
    command::LaunchCommand,
    error::{Error, Result},
};

pub fn spawn_command(command: LaunchCommand) -> Result<Child> {
    let program = command.program();
    if program.as_os_str().is_empty() {
        return Err(Error::EmptyExecutable);
    }

    let mut child = Command::new(&program);
    child.args(command.arguments());

    if let Some(cwd) = &command.cwd {
        child.current_dir(cwd);
    }

    for (key, value) in &command.env {
        child.env(key, value);
    }

    child
        .spawn()
        .map_err(|source| Error::Spawn { program, source })
}
