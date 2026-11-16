pub mod microsoft;
pub mod offline;
#[cfg(feature = "yggdrasil-auth")]
pub mod yggdrasil;

use crate::auth::{accounts::Account, error::Result};

pub trait Authorizer {
    type Account: Account;

    /// Stable persistence key and account storage path segment.
    ///
    /// Implementations must return a non-empty, single, filesystem-safe
    /// segment, for example `[a-z0-9_-]+`. Do not use path separators, `..`,
    /// drive prefixes, or Windows-reserved names. This value must not change
    /// once accounts are stored.
    fn id() -> &'static str;

    /// Human-readable name for user interfaces.
    fn name() -> &'static str {
        "Authorizer"
    }

    fn authorize(&self) -> impl Future<Output = Result<Self::Account>>;
}
