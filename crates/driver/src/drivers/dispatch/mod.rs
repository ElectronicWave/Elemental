mod installer;
mod profiled;

use elemental_core::minecraft::MinecraftVersionId;

use crate::{driver::DriverDescriptor, loader_version::LoaderVersionId};

pub use installer::{InstallerDriver, InstallerPrepared};
pub use profiled::{ProfiledDriver, ProfiledPrepared};

#[derive(Debug, Clone)]
pub struct LoaderRequest {
    pub descriptor: DriverDescriptor,
    pub game_version: MinecraftVersionId,
    pub loader_version: LoaderVersionId,
}
