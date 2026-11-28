use elemental_core::auth::accounts::AccountId;

use crate::texture::{CapeId, SkinId};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("instance '{name}' not found")]
    InstanceNotFound { name: String },

    #[error("unsupported driver '{id}'")]
    UnsupportedDriver { id: String },

    #[error("driver '{id}' is already registered")]
    DriverAlreadyRegistered { id: String },

    #[error("driver '{id}' does not support loader versions")]
    LoaderVersionsUnsupported { id: String },

    #[error("driver '{id}' requires a loader version")]
    LoaderVersionRequired { id: String },

    #[error("driver '{id}' does not accept a loader version")]
    UnexpectedLoaderVersion { id: String },

    #[error(transparent)]
    Addon(#[from] elemental_addon::Error),

    #[error("addon '{id}' is already registered")]
    AddonAlreadyRegistered { id: String },

    #[error("addon '{id}' is not registered")]
    AddonNotRegistered { id: String },

    #[error("addon '{id}' is incompatible with this instance: {reason}")]
    AddonIncompatible { id: String, reason: String },

    #[error(transparent)]
    Driver(#[from] elemental_driver::error::Error),

    #[error(transparent)]
    Infra(#[from] elemental_infra::error::Error),

    #[error(transparent)]
    Launcher(#[from] elemental_core::launcher::Error),

    #[error(transparent)]
    Storage(#[from] elemental_core::storage::Error),

    #[error(transparent)]
    Shared(#[from] elemental_shared::error::Error),

    #[error(transparent)]
    Auth(#[from] elemental_core::auth::Error),

    #[error("account '{account_id}' not found")]
    AccountNotFound { account_id: AccountId },

    #[error("account '{account_id}' uses unknown authorizer '{authorizer}'")]
    UnknownAuthorizer {
        account_id: AccountId,
        authorizer: String,
    },

    #[error("encode account '{account_id}' for authorizer '{authorizer}' failed")]
    AccountEncode {
        account_id: AccountId,
        authorizer: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("decode account '{account_id}' for authorizer '{authorizer}' failed")]
    AccountDecode {
        account_id: AccountId,
        authorizer: String,
        #[source]
        source: serde_json::Error,
    },

    #[error(
        "account file does not match its index entry: expected '{expected_authorizer}/{expected_id}', found '{actual_authorizer}/{actual_id}'"
    )]
    AccountMismatch {
        expected_id: AccountId,
        expected_authorizer: String,
        actual_id: AccountId,
        actual_authorizer: String,
    },

    #[error("skin '{skin_id}' not found")]
    SkinNotFound { skin_id: SkinId },

    #[error("skin file for '{skin_id}' not found at '{}'", path.display())]
    SkinFileNotFound {
        skin_id: SkinId,
        path: std::path::PathBuf,
    },

    #[error("skin '{name}' is not a PNG file")]
    SkinNotPng { name: String },

    #[error("cape '{cape_id}' not found")]
    CapeNotFound { cape_id: CapeId },

    #[error("cape file for '{cape_id}' not found at '{}'", path.display())]
    CapeFileNotFound {
        cape_id: CapeId,
        path: std::path::PathBuf,
    },

    #[error("cape '{name}' is not a PNG file")]
    CapeNotPng { name: String },

    #[error("texture name cannot be empty")]
    TextureNameEmpty,

    #[error(transparent)]
    HttpClient(#[from] elemental_core::http::HttpClientError),
}

pub type Result<T> = std::result::Result<T, Error>;
