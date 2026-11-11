use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::auth::accounts::{Account, AccountId};
use crate::auth::authorizers::microsoft::MICROSOFT_AUTHORIZER_ID;
use crate::auth::credential::{Credential, CredentialKind};
use crate::auth::error::Result;
use crate::auth::texture::{Cape, ProfileTextures, Skin, SkinModel, TextureType};

const REFRESH_SKEW_SECONDS: u64 = 300;

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
        Ok(Box::new(
            crate::auth::authorizers::microsoft::refresh(self).await?,
        ))
    }

    async fn textures(&self) -> Result<ProfileTextures> {
        profile_textures(
            crate::auth::authorizers::microsoft::minecraft_profile(&self.access_token).await?,
        )
    }

    async fn upload_skin(&self, image: Vec<u8>, model: SkinModel) -> Result<()> {
        crate::auth::authorizers::microsoft::upload_skin(&self.access_token, image, model).await
    }

    async fn delete_skin(&self) -> Result<()> {
        crate::auth::authorizers::microsoft::delete_skin(&self.access_token).await
    }

    async fn delete_cape(&self) -> Result<()> {
        crate::auth::authorizers::microsoft::select_cape(&self.access_token, None).await
    }

    async fn select_cape(&self, cape_id: Option<String>) -> Result<()> {
        crate::auth::authorizers::microsoft::select_cape(&self.access_token, cape_id).await
    }
}

pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn profile_textures(
    profile: crate::auth::authorizers::microsoft::MinecraftProfileResponse,
) -> Result<ProfileTextures> {
    Ok(ProfileTextures {
        skin: active_skin(profile.skins)?,
        capes: profile.capes.into_iter().map(cape_from).collect(),
        uploadable: vec![TextureType::Skin],
    })
}

fn active_skin(
    skins: Vec<crate::auth::authorizers::microsoft::MinecraftTexture>,
) -> Result<Option<Skin>> {
    skins
        .into_iter()
        .find(|texture| texture.state.eq_ignore_ascii_case("ACTIVE"))
        .map(skin_from)
        .transpose()
}

fn skin_from(texture: crate::auth::authorizers::microsoft::MinecraftTexture) -> Result<Skin> {
    let variant = texture.variant.unwrap_or_default();
    let model = match variant.to_ascii_lowercase().as_str() {
        "" | "classic" => SkinModel::Classic,
        "slim" => SkinModel::Slim,
        _ => return Err(crate::auth::error::Error::TextureModelUnknown { model: variant }),
    };
    Ok(Skin {
        url: texture.url,
        model,
    })
}

fn cape_from(texture: crate::auth::authorizers::microsoft::MinecraftTexture) -> Cape {
    Cape {
        id: texture.id,
        alias: texture.alias,
        url: texture.url,
        active: texture.state.eq_ignore_ascii_case("ACTIVE"),
    }
}
