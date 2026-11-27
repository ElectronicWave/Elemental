mod custom_patch;
mod jvm_agent;
#[cfg(feature = "liteloader")]
mod liteloader;

pub use custom_patch::CustomPatchAddon;
pub use jvm_agent::JvmAgentAddon;
#[cfg(feature = "liteloader")]
pub use liteloader::{
    LITELOADER_ADDON_ID, LiteLoaderAddon,
    catalog::LiteLoaderCatalog,
    source::{LiteLoaderEndpoints, LiteLoaderOrigin, LiteLoaderRelease, LiteLoaderSource},
};
