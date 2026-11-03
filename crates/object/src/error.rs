use thiserror::Error;

/// Errors returned by the object pool facade.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot get object `{type_name}` from pool")]
    ObjectNotFound { type_name: &'static str },

    #[error("Cannot fulfill object `{type_name}`: no waiting entry exists")]
    NoWaitingEntry { type_name: &'static str },

    #[error("Cannot fulfill object `{type_name}`: the entry is closed; use provide instead")]
    EntryClosed { type_name: &'static str },
}

/// Result type used by the object pool facade.
pub type Result<T> = std::result::Result<T, Error>;
