pub mod account;
#[cfg(feature = "yggdrasil-auth")]
mod authlib_injector;
mod builder;
mod catalog;
pub mod error;
mod launcher;
#[cfg(feature = "offline-server")]
pub mod offline_server;
mod origin;
mod request;
mod result;
mod spec;

pub use account::{AccountIndex, AccountRef, AccountRegistry, AccountStore};
#[cfg(feature = "yggdrasil-auth")]
pub use authlib_injector::{AuthlibInjector, AuthlibInjectorSource};
pub use builder::LauncherBuilder;
pub use catalog::DriverCatalog;
pub use elemental_core::auth::credential::Credential;
pub use elemental_core::auth::texture::{Cape, ProfileTextures, Skin, SkinModel, TextureType};
pub use elemental_core::launcher::command::LaunchCommand;
pub use elemental_infra::downloader::{DownloadProgress, DownloadSession};
pub use launcher::Launcher;
pub use origin::DriverOrigins;
pub use request::{LaunchOptions, LaunchResolution, QuickPlayOptions, RuntimeOptions};
pub use result::{
    Instance, LaunchedInstance, PreparedInstance, PreparedInstanceInfo, PreparedInstanceMetadata,
};
pub use spec::{DriverSpec, LoaderSpec, VanillaSpec};
