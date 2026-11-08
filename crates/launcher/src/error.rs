#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("instance '{name}' not found")]
    InstanceNotFound { name: String },

    #[error("unsupported driver '{id}'")]
    UnsupportedDriver { id: String },

    #[error("driver '{id}' does not support loader versions")]
    LoaderVersionsUnsupported { id: String },

    #[error("driver '{id}' does not accept a loader version")]
    UnexpectedLoaderVersion { id: String },

    #[error("resolved launcher driver does not support '{id}' prepare flow")]
    UnsupportedDriverFlow { id: String },

    #[error(transparent)]
    Driver(#[from] elemental_driver::error::Error),

    #[error(transparent)]
    Infra(#[from] elemental_infra::error::Error),

    #[error(transparent)]
    Launcher(#[from] elemental_core::launcher::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
