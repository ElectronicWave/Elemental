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
use elemental::core::runtime::{
    provider::{RuntimeProvider, default_providers, with_runtime_providers},
    providers::custom::new_custom_provider,
};
use std::sync::Arc;

use crate::cli::Cli;
use crate::config::LaunchpadCommand;
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
    if let LaunchpadCommand::Launch(config) = &command
        && !config.runtime_paths.is_empty()
    {
        let mut providers = Vec::<Arc<dyn RuntimeProvider>>::new();
        providers.push(Arc::new(new_custom_provider(config.runtime_paths.clone())?));
        providers.extend(default_providers());
        with_runtime_providers(providers)?;
    }
    commands::run(command).await
}
