mod cli;
mod commands;
mod config;
mod diagnostics;

use anyhow::Result;
use clap::Parser;

use crate::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Cli::parse().into_demo_config();
    commands::run(config).await
}
