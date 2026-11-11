pub mod microsoft;
pub mod offline;
#[cfg(feature = "yggdrasil-auth")]
pub mod yggdrasil;

use crate::auth::{accounts::Account, error::Result};

pub trait Authorizer {
    type Account: Account;

    /// Stable persistence key. This must not change once accounts are stored.
    fn id() -> &'static str;

    /// Human-readable name for user interfaces.
    fn name() -> &'static str {
        "Authorizer"
    }

    fn authorize(&self) -> impl Future<Output = Result<Self::Account>>;
}
