use anyhow::{Context, Result, bail};
use tokio::process::{Child, Command};

pub fn spawn_command(args: Vec<String>) -> Result<Child> {
	let (exe, process_args) = args
		.split_first()
		.context("launch args is empty")?;
	if exe.is_empty() {
		bail!("launch executable is empty");
	}

	Ok(Command::new(exe).args(process_args).spawn()?)
}
