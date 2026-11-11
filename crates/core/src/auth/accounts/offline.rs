use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::accounts::{Account, AccountId};
use crate::auth::authorizers::offline::OFFLINE_AUTHORIZER_ID;
use crate::auth::credential::{Credential, CredentialKind};
use crate::auth::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineAccount {
    pub account_id: AccountId,
    pub username: String,
    pub uuid: String,
}

impl OfflineAccount {
    pub fn new(username: String) -> Self {
        let uuid = offline_uuid(&username);
        Self {
            account_id: AccountId::derive(OFFLINE_AUTHORIZER_ID, &username),
            username,
            uuid: uuid.to_string(),
        }
    }
}

#[async_trait]
impl Account for OfflineAccount {
    fn account_id(&self) -> AccountId {
        self.account_id
    }

    fn authorizer_id(&self) -> &'static str {
        OFFLINE_AUTHORIZER_ID
    }

    fn display_name(&self) -> &str {
        &self.username
    }

    async fn credential(&self) -> Result<Credential> {
        Ok(Credential {
            username: self.username.clone(),
            uuid: self.uuid.clone(),
            access_token: String::new(),
            kind: CredentialKind::Legacy,
            xuid: None,
            client_id: None,
            #[cfg(feature = "yggdrasil-auth")]
            auth_server: None,
        })
    }

    async fn refresh(&self) -> Result<Box<dyn Account>> {
        Ok(Box::new(self.clone()))
    }
}

fn offline_uuid(username: &str) -> Uuid {
    use md5::{Digest, Md5};

    let offline_name = format!("OfflinePlayer:{username}");
    let mut hasher = Md5::new();
    hasher.update(offline_name);
    let mut buffer = hasher.finalize();
    buffer[6] = (buffer[6] & 0x0f) | 0x30;
    buffer[8] = (buffer[8] & 0x3f) | 0x80;
    Uuid::from_bytes(buffer.into())
}
