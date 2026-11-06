macro_rules! dispatch_driver_await {
    ($this:expr, |$driver:ident| $body:expr => [$($variant:ident),+ $(,)?]) => {
        match $this {
            $(Self::$variant($driver) => $body.await,)+
        }
    };
}

mod installer;
mod profiled;

use elemental_core::minecraft::MinecraftVersionId;

use crate::loader_version::LoaderVersionId;

pub use installer::InstallerDriver;
pub use profiled::ProfiledDriver;

#[derive(Debug, Clone)]
pub struct LoaderRequest {
    pub game_version: MinecraftVersionId,
    pub loader_version: LoaderVersionId,
}
