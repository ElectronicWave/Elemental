#[cfg(feature = "authlib-injector")]
mod authlib_injector;
mod custom_patch;
mod jvm_agent;
#[cfg(feature = "liteloader")]
mod liteloader;

#[cfg(feature = "authlib-injector")]
pub use authlib_injector::{
    AUTHLIB_INJECTOR_ADDON_ID, AUTHLIB_INJECTOR_OFFICIAL_URL, AuthlibInjector,
    AuthlibInjectorAddon, AuthlibInjectorEndpoints, AuthlibInjectorSource,
};
pub use custom_patch::CustomPatchAddon;
pub use jvm_agent::JvmAgentAddon;
#[cfg(feature = "liteloader")]
pub use liteloader::{
    LITELOADER_ADDON_ID, LiteLoaderAddon,
    catalog::LiteLoaderCatalog,
    source::{LiteLoaderEndpoints, LiteLoaderOrigin, LiteLoaderRelease, LiteLoaderSource},
};
