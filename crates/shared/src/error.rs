use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[cfg(feature = "json")]
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[cfg(feature = "toml")]
    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[cfg(feature = "toml")]
    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("no valid base path is available")]
    BasePathUnavailable,

    #[error("stored version {stored} is newer than supported version {supported}")]
    StoredVersionNewer { stored: usize, supported: usize },

    #[error("migrator encoded version {encoded}, expected target version {target}")]
    EncodedVersionMismatch { encoded: usize, target: usize },
}

pub type Result<T> = std::result::Result<T, Error>;
