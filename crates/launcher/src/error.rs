#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("unsupported driver '{id}'")]
    UnsupportedDriver { id: String },

    #[error("resolved launcher driver does not support '{id}' prepare flow")]
    UnsupportedDriverFlow { id: String },

    #[error(transparent)]
    Driver(#[from] elemental_driver::error::Error),

    #[error(transparent)]
    Launcher(#[from] elemental_core::launcher::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
