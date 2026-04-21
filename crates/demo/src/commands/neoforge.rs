use anyhow::Result;
use elemental::driver::drivers::neoforge::driver::NeoForgeDriver;

use crate::{commands::run_installer_family_demo, config::DemoConfig};

pub async fn run(config: DemoConfig) -> Result<()> {
    let driver = NeoForgeDriver::with_defaults()?;

    run_installer_family_demo(config, "neoforge", &driver).await
}
