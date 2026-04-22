use anyhow::Result;
use elemental::driver::drivers::rift::driver::RiftDriverFamily;

use crate::{commands::run_profiled_version_json_demo, config::DemoConfig};

pub async fn run(config: DemoConfig) -> Result<()> {
    let driver = RiftDriverFamily::new_driver_with_defaults()?;

    run_profiled_version_json_demo(config, "rift", &driver).await
}
