use anyhow::Result;
use elemental::driver::{
    drivers::liteloader::driver::LiteLoaderDriverFamily,
    families::version_json::ProfiledVersionJsonFamilyExt,
};

use crate::{commands::run_profiled_version_json_demo, config::DemoConfig};

pub async fn run(config: DemoConfig) -> Result<()> {
    let driver = LiteLoaderDriverFamily.build_driver_with_defaults()?;

    run_profiled_version_json_demo(config, "liteloader", &driver).await
}
