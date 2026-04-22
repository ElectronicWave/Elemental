use anyhow::Result;
use elemental::driver::drivers::cleanroom::driver::CleanroomDriver;

use crate::{commands::run_installer_family_demo, config::DemoConfig};

pub async fn run(config: DemoConfig) -> Result<()> {
    let driver = CleanroomDriver::with_defaults()?;

    run_installer_family_demo(config, "cleanroom", &driver).await
}
