use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("launch executable is empty")]
    EmptyExecutable,

    #[error("failed to spawn '{}': {source}", program.display())]
    Spawn {
        program: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
