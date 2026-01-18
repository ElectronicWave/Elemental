use crate::auth::{authorizer::Authorizer, credential::UserCredential};
use anyhow::Result;
use md5::{Digest, Md5};
use uuid::Uuid;

pub struct OfflineAuthorizer {
    pub username: String,
}

impl Authorizer for OfflineAuthorizer {
    async fn authorize(&self) -> Result<UserCredential> {
        let offline_name = format!("OfflinePlayer:{}", self.username);
        let mut hasher = Md5::new();
        hasher.update(offline_name);
        let mut buffer = hasher.finalize();
        buffer[6] = (buffer[6] & 0x0f) | 0x30;
        buffer[8] = (buffer[8] & 0x3f) | 0x80;

        Ok(UserCredential {
            uuid: Uuid::from_bytes(buffer.into()).to_string(),
            access_token: "".to_string(),
        })
    }

    fn name() -> &'static str {
        "Offline"
    }
}
