//! Unified facade for the Elemental crates.
//!
//! Each module re-exports one workspace crate. The [`prelude`] module collects
//! the types used by a typical launcher integration.

#[cfg(feature = "core")]
pub use elemental_core as core;
#[cfg(feature = "driver")]
pub use elemental_driver as driver;
#[cfg(feature = "infra")]
pub use elemental_infra as infra;
#[cfg(feature = "launcher")]
pub use elemental_launcher as launcher;
#[cfg(feature = "object")]
pub use elemental_object as object;
#[cfg(feature = "schema")]
pub use elemental_schema as schema;
#[cfg(feature = "shared")]
pub use elemental_shared as shared;

pub mod prelude {
    #[cfg(feature = "core")]
    pub use crate::core::{
        auth::{
            accounts::{Account, AccountId, microsoft::MicrosoftAccount, offline::OfflineAccount},
            authorizers::{
                Authorizer,
                microsoft::{MICROSOFT_AUTHORIZER_ID, MicrosoftAuthorizer},
                offline::{OFFLINE_AUTHORIZER_ID, OfflineAuthorizer},
            },
            credential::{Credential, CredentialKind},
            texture::{Cape, ProfileTextures, Skin, SkinModel, TextureType},
        },
        launcher::command::LaunchCommand,
        minecraft::MinecraftVersionId,
        runtime::{
            RuntimeResolver, RuntimeValidationMode,
            distribution::Distribution,
            provider::{RuntimeProvider, RuntimeProviderList, default_providers},
        },
    };

    #[cfg(feature = "yggdrasil-auth")]
    pub use crate::core::auth::{
        accounts::yggdrasil::{YggdrasilAccount, YggdrasilProfile},
        authorizers::yggdrasil::{YGGDRASIL_AUTHORIZER_ID, YggdrasilAuthorizer},
    };

    #[cfg(feature = "driver")]
    pub use crate::driver::{
        catalog::{GameVersion, LoaderRelease},
        driver::DriverDescriptor,
        families::version_json::{
            BaseInstanceLayout, BaseRootLayout, VersionJsonInstanceLayout, VersionJsonRootLayout,
        },
        loader_version::LoaderVersionId,
    };

    #[cfg(feature = "infra")]
    pub use crate::infra::downloader::{DownloadProgress, DownloadSession, ElementalDownloader};

    #[cfg(feature = "launcher")]
    pub use crate::launcher::{
        AccountIndex, AccountRef, AccountRegistry, AccountStore, DriverCatalog, DriverOrigins,
        DriverSpec, Instance, LaunchOptions, LaunchResolution, LaunchedInstance, Launcher,
        LauncherBuilder, LoaderSpec, PreparedInstance, PreparedInstanceInfo,
        PreparedInstanceMetadata, QuickPlayOptions, RuntimeOptions, VanillaSpec,
    };
}
