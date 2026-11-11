use crate::auth::accounts::offline::OfflineAccount;
use crate::auth::authorizers::Authorizer;
use crate::auth::error::Result;

pub const OFFLINE_AUTHORIZER_ID: &str = "offline";

pub struct OfflineAuthorizer {
    pub username: String,
}

impl Authorizer for OfflineAuthorizer {
    type Account = OfflineAccount;

    async fn authorize(&self) -> Result<Self::Account> {
        Ok(OfflineAccount::new(self.username.clone()))
    }

    fn id() -> &'static str {
        OFFLINE_AUTHORIZER_ID
    }

    fn name() -> &'static str {
        "Offline"
    }
}
