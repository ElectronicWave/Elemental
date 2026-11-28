elemental_core::require_tls_backend!();

pub mod account;
mod addon;
mod builder;
pub mod cape;
mod catalog;
pub mod error;
mod launcher;
#[cfg(feature = "offline-server")]
pub mod offline_server;
mod origin;
mod registration;
mod request;
mod result;
pub mod skin;
mod spec;
mod texture;

pub use account::{AccountIndex, AccountListing, AccountRef, AccountRegistry, AccountStore};
pub use addon::{AddonRegistry, AddonSpec};
pub use builder::LauncherBuilder;
pub use cape::{CapeAsset, CapeEntry, CapeId, CapeStore, SelectedCape};
pub use catalog::DriverCatalog;
pub use elemental_addon::{
    self as addon_kit, Addon, AddonCatalogQuery, AddonCompatibility, AddonContext, AddonDescriptor,
    AddonLaunchArgs, AddonLaunchContext, AddonModify, AddonOrigin, AddonPatchMode,
    AddonPrepareOutput, AddonPrepareRequest, AddonRelease, CustomPatchAddon, GameVersion,
    InstalledAddon, JvmAgentAddon, MetadataPatch, MetadataPatchStepDiff, PatchMode,
    PatchModeSupport, PatchTrace,
};
#[cfg(feature = "yggdrasil-auth")]
pub use elemental_addon::{
    AUTHLIB_INJECTOR_ADDON_ID, AuthlibInjector, AuthlibInjectorAddon, AuthlibInjectorEndpoints,
    AuthlibInjectorSource,
};
#[cfg(feature = "addon-liteloader")]
pub use elemental_addon::{LITELOADER_ADDON_ID, LiteLoaderAddon};
pub use elemental_core::auth::credential::Credential;
pub use elemental_core::auth::texture::{
    Cape, ProfileTextures, Skin, SkinModel, SkinPixels, TextureType,
};
pub use elemental_core::http::{HttpClientConfig, HttpClientError, SwappableClient};
pub use elemental_core::launcher::command::LaunchCommand;
pub use elemental_driver::driver::PrepareStage;
pub use elemental_infra::downloader::{DownloadProgress, DownloadSession};
pub use error::{Error, Result};
pub use launcher::{ActionScope, Launcher};
pub use origin::DriverOrigins;
pub use registration::{DriverRegistration, DriverRegistry};
pub use request::{LaunchOptions, LaunchResolution, QuickPlayOptions, RuntimeOptions};
pub use result::{
    InspectReport, Instance, InstanceInspectionFailure, LaunchedInstance, PreparedInstance,
    PreparedInstanceInfo, PreparedInstanceMetadata, RemoveInstanceOutcome,
};
pub use skin::{AccountSkinExt, LocalSkin, SelectedSkin, SkinAsset, SkinEntry, SkinId, SkinStore};
pub use spec::DriverSpec;
