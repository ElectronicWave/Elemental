mod cli;
mod commands;
mod config;
mod defaults;
mod diagnostics;
mod interactive;
mod invocation;
mod progress;

use anyhow::Result;
use clap::Parser;

use crate::cli::Cli;
use crate::invocation::Invocation;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let command = match Cli::parse().into_invocation()? {
        Invocation::Command(command) => command,
        Invocation::Interactive {
            storage_root,
            local_only,
        } => interactive::prompt_command(storage_root, local_only).await?,
    };
    commands::run(command).await
}
