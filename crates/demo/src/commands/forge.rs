use anyhow::Result;
use elemental::driver::drivers::forge::driver::ForgeDriver;

use crate::{commands::run_installer_family_demo, config::DemoConfig};

pub async fn run(config: DemoConfig) -> Result<()> {
    let driver = ForgeDriver::with_defaults()?;

    run_installer_family_demo(config, "forge", &driver).await
}
