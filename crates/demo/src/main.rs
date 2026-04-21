mod cli;
mod commands;
mod config;
mod diagnostics;

use anyhow::Result;
use clap::Parser;
use elemental::core::runtime::{
    provider::{RuntimeProvider, default_providers, with_runtime_providers},
    providers::custom::new_custom_provider,
};
use std::sync::Arc;

use crate::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Cli::parse().into_demo_config();
    if !config.runtime_paths.is_empty() {
        let mut providers = Vec::<Arc<dyn RuntimeProvider>>::new();
        providers.push(Arc::new(new_custom_provider(config.runtime_paths.clone())?));
        providers.extend(default_providers());
        with_runtime_providers(providers)?;
    }
    commands::run(config).await
}
