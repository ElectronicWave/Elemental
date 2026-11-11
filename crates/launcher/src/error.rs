use elemental_core::auth::accounts::AccountId;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("instance '{name}' not found")]
    InstanceNotFound { name: String },

    #[error("unsupported driver '{id}'")]
    UnsupportedDriver { id: String },

    #[error("driver '{id}' does not support loader versions")]
    LoaderVersionsUnsupported { id: String },

    #[error("driver '{id}' requires a loader version")]
    LoaderVersionRequired { id: String },

    #[error("driver '{id}' does not accept a loader version")]
    UnexpectedLoaderVersion { id: String },

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

    #[cfg(feature = "yggdrasil-auth")]
    #[error(transparent)]
    AuthlibInjectorDistribution(#[from] nidhogg::error::NidhoggError),

    #[cfg(feature = "yggdrasil-auth")]
    #[error("authlib-injector download completed but '{}' is missing", path.display())]
    AuthlibInjectorMissing { path: std::path::PathBuf },

    #[cfg(feature = "yggdrasil-auth")]
    #[error(
        "authlib-injector checksum mismatch for '{}': expected {expected}, got {actual}",
        path.display()
    )]
    AuthlibInjectorChecksum {
        path: std::path::PathBuf,
        expected: String,
        actual: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
