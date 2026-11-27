use std::{io, path::PathBuf};

use elemental_driver::families::version_json::PatchMode;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Driver(#[from] elemental_driver::error::Error),

    #[error(transparent)]
    Origin(#[from] elemental_core::url::OriginError),

    #[cfg(feature = "liteloader")]
    #[error(transparent)]
    HttpRequest(#[from] elemental_core::http::HttpRequestError),

    #[error(transparent)]
    Infra(#[from] elemental_infra::error::Error),

    #[error(transparent)]
    Shared(#[from] elemental_shared::error::Error),

    #[error(transparent)]
    Storage(#[from] elemental_core::storage::Error),

    #[error("addon '{id}' is incompatible with this instance: {reason}")]
    Incompatible { id: String, reason: String },

    #[error("addon '{id}' requires an explicit version")]
    VersionRequired { id: String },

    #[error("managed addon file path escapes instance directory: '{}'", path.display())]
    UnsafeManagedPath { path: PathBuf },

    #[error("addon '{id}' does not support {requested:?} patch mode (supports {supported})")]
    PatchModeUnsupported {
        id: String,
        requested: PatchMode,
        supported: String,
    },

    #[error(
        "addon '{id}' cannot use physical patching when instance policy is {instance_policy:?}"
    )]
    PatchModeDisallowedByInstance {
        id: String,
        instance_policy: PatchMode,
    },

    #[error("addon '{id}' is not installed on this instance")]
    AddonNotInstalled { id: String },

    #[error("cannot reorder addon '{id}': not installed on this instance")]
    UnknownAddonInOrder { id: String },

    #[cfg(feature = "liteloader")]
    #[error("can't find LiteLoader Minecraft version '{game_version}'")]
    LiteLoaderMinecraftVersionNotFound { game_version: String },

    #[cfg(feature = "liteloader")]
    #[error(
        "can't find LiteLoader loader version '{loader_version}' for Minecraft '{game_version}'"
    )]
    LiteLoaderLoaderVersionNotFound {
        loader_version: String,
        game_version: String,
    },

    #[cfg(feature = "liteloader")]
    #[error("LiteLoader library '{library}' does not declare a supported repository")]
    LiteLoaderLibraryRepositoryMissing { library: String },

    #[cfg(feature = "liteloader")]
    #[error("LiteLoader snapshot library '{library}' does not declare a repository")]
    LiteLoaderSnapshotRepositoryMissing { library: String },
}

pub type Result<T> = std::result::Result<T, Error>;
