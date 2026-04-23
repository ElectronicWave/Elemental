use elemental_driver::families::version_json::VersionJsonLaunchConfig;

use crate::spec::DriverSpec;

pub type LaunchOptions = VersionJsonLaunchConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareInstanceRequest {
    pub instance_name: String,
    pub driver: DriverSpec,
}
