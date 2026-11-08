pub mod discovery;
pub mod distribution;
pub mod error;
pub mod provider;
pub mod providers;
mod resolver;

pub use error::{Error, Result};
pub use resolver::RuntimeResolver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeValidationMode {
    Strict,
    Disabled,
}
