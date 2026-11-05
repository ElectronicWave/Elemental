use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(
        "layout '{layout}' is missing resource {resource} for '{}'",
        root.display()
    )]
    ResourceNotFound {
        layout: &'static str,
        resource: String,
        root: PathBuf,
    },

    #[error("failed to {operation} '{}': {source}", path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
