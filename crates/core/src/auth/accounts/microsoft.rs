use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::auth::accounts::{Account, AccountId};
use crate::auth::credential::{Credential, CredentialKind};
use crate::auth::error::Result;
use crate::auth::microsoft_protocol::{
    ExchangedMinecraftSession, delete_skin, minecraft_profile, profile_textures,
    refresh_microsoft_session, select_cape, unix_now, upload_skin,
};
use crate::auth::texture::{ProfileTextures, SkinModel};

const REFRESH_SKEW_SECONDS: u64 = 300;

pub const MICROSOFT_AUTHORIZER_ID: &str = "microsoft";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftAccount {
    pub account_id: AccountId,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
    pub client_id: String,
    pub xuid: Option<String>,
    pub expires_at: u64,
}

impl MicrosoftAccount {
    pub(crate) fn from_session(client_id: &str, session: ExchangedMinecraftSession) -> Self {
        Self {
            account_id: AccountId::derive(MICROSOFT_AUTHORIZER_ID, &session.uuid),
            username: session.username,
            uuid: session.uuid,
            access_token: session.access_token,
            refresh_token: session.refresh_token,
            client_id: client_id.to_owned(),
            xuid: session.xuid,
            expires_at: session.expires_at,
        }
    }
}

#[async_trait]
impl Account for MicrosoftAccount {
    fn account_id(&self) -> AccountId {
        self.account_id
    }

    fn authorizer_id(&self) -> &'static str {
        MICROSOFT_AUTHORIZER_ID
    }

    fn display_name(&self) -> &str {
        &self.username
    }

    fn needs_refresh(&self) -> bool {
        unix_now().saturating_add(REFRESH_SKEW_SECONDS) >= self.expires_at
    }

    async fn credential(&self) -> Result<Credential> {
        Ok(Credential {
            username: self.username.clone(),
            uuid: self.uuid.clone(),
            access_token: self.access_token.clone(),
            kind: CredentialKind::Msa,
            xuid: self.xuid.clone(),
            client_id: Some(self.client_id.clone()),
            #[cfg(feature = "yggdrasil-auth")]
            auth_server: None,
        })
    }

    async fn refresh(&self) -> Result<Box<dyn Account>> {
        let session =
            refresh_microsoft_session(&self.client_id, &self.refresh_token, self.xuid.clone())
                .await?;
        Ok(Box::new(Self::from_session(&self.client_id, session)))
    }

    async fn textures(&self) -> Result<ProfileTextures> {
        profile_textures(minecraft_profile(&self.access_token).await?)
    }

    async fn upload_skin(&self, image: Vec<u8>, model: SkinModel) -> Result<()> {
        upload_skin(&self.access_token, image, model).await
    }

    async fn delete_skin(&self) -> Result<()> {
        delete_skin(&self.access_token).await
    }

    async fn delete_cape(&self) -> Result<()> {
        select_cape(&self.access_token, None).await
    }

    async fn select_cape(&self, cape_id: Option<String>) -> Result<()> {
        select_cape(&self.access_token, cape_id).await
    }
}
