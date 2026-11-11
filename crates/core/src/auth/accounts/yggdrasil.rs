use async_trait::async_trait;
use nidhogg::{
    client::Nidhogg,
    request::texture::TextureType as NidhoggTextureType,
    types::{
        profile::{Profile, ProfilePropertyError},
        textures::{ModelType, Texture},
    },
};
use serde::{Deserialize, Serialize};

use crate::auth::accounts::{Account, AccountId};
use crate::auth::authorizers::yggdrasil::{YGGDRASIL_AUTHORIZER_ID, YggdrasilAuthorizer};
use crate::auth::credential::{Credential, CredentialKind};
use crate::auth::error::{Error, Result};
use crate::auth::texture::{
    Cape, ProfileTextures, Skin, SkinModel, TextureType as AccountTextureType,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YggdrasilProfile {
    pub id: String,
    pub name: String,
}

impl From<&Profile> for YggdrasilProfile {
    fn from(profile: &Profile) -> Self {
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YggdrasilAccount {
    pub account_id: AccountId,
    pub base_url: String,
    pub user_id: Option<String>,
    pub preferred_language: Option<String>,
    pub access_token: String,
    pub client_token: String,
    pub selected_profile: YggdrasilProfile,
}

impl YggdrasilAccount {
    fn client(&self) -> Result<Nidhogg> {
        Nidhogg::new(self.base_url.trim_end_matches('/'))
            .map_err(|source| Error::Yggdrasil { source })
    }

    async fn upload(
        &self,
        texture_type: NidhoggTextureType,
        image: Vec<u8>,
        file_name: &str,
        model: Option<ModelType>,
    ) -> Result<()> {
        self.client()?
            .upload_texture(
                self.selected_profile.id.clone(),
                self.access_token.clone(),
                texture_type,
                image,
                file_name.to_owned(),
                model,
            )
            .await
            .map_err(|source| Error::Yggdrasil { source })
    }

    async fn delete(&self, texture_type: NidhoggTextureType) -> Result<()> {
        self.client()?
            .delete_texture(
                self.selected_profile.id.clone(),
                self.access_token.clone(),
                texture_type,
            )
            .await
            .map_err(|source| Error::Yggdrasil { source })
    }
}

#[async_trait]
impl Account for YggdrasilAccount {
    fn account_id(&self) -> AccountId {
        self.account_id
    }

    fn authorizer_id(&self) -> &'static str {
        YGGDRASIL_AUTHORIZER_ID
    }

    fn display_name(&self) -> &str {
        &self.selected_profile.name
    }

    async fn credential(&self) -> Result<Credential> {
        Ok(Credential {
            username: self.selected_profile.name.clone(),
            uuid: self.selected_profile.id.clone(),
            access_token: self.access_token.clone(),
            kind: CredentialKind::Mojang,
            xuid: None,
            client_id: None,
            auth_server: Some(self.base_url.clone()),
        })
    }

    async fn refresh(&self) -> Result<Box<dyn Account>> {
        Ok(Box::new(YggdrasilAuthorizer::refresh_account(self).await?))
    }

    async fn textures(&self) -> Result<ProfileTextures> {
        let profile = self
            .client()?
            .get_profile(self.selected_profile.id.clone(), false)
            .await
            .map_err(|source| Error::Yggdrasil { source })?;
        profile_textures(&profile)
    }

    async fn upload_skin(&self, image: Vec<u8>, model: SkinModel) -> Result<()> {
        let model = match model {
            SkinModel::Classic => None,
            SkinModel::Slim => Some(ModelType::SLIM),
        };
        self.upload(NidhoggTextureType::SKIN, image, "skin.png", model)
            .await
    }

    async fn delete_skin(&self) -> Result<()> {
        self.delete(NidhoggTextureType::SKIN).await
    }

    async fn upload_cape(&self, image: Vec<u8>) -> Result<()> {
        self.upload(NidhoggTextureType::CAPE, image, "cape.png", None)
            .await
    }

    async fn delete_cape(&self) -> Result<()> {
        self.delete(NidhoggTextureType::CAPE).await
    }
}

fn profile_textures(profile: &Profile) -> Result<ProfileTextures> {
    let textures = optional_property(profile.get_textures())?;
    let skin = textures
        .as_ref()
        .and_then(|textures| textures.textures.skin.as_ref())
        .map(skin_from)
        .transpose()?;
    let capes = textures
        .and_then(|textures| textures.textures.cape)
        .map(cape_from)
        .into_iter()
        .collect();
    let uploadable = optional_property(profile.get_uploadable_textures())?
        .unwrap_or_default()
        .into_iter()
        .filter_map(uploadable_texture)
        .collect();

    Ok(ProfileTextures {
        skin,
        capes,
        uploadable,
    })
}

fn optional_property<T>(result: std::result::Result<T, ProfilePropertyError>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(ProfilePropertyError::NoPropertyFound(_)) => Ok(None),
        Err(source) => Err(Error::YggdrasilTexture { source }),
    }
}

fn skin_from(texture: &Texture) -> Result<Skin> {
    let model = match texture.metadata.as_ref().map(|metadata| &metadata.model) {
        Some(ModelType::SLIM) => SkinModel::Slim,
        Some(ModelType::DEFAULT) | None => SkinModel::Classic,
    };
    Ok(Skin {
        url: texture.url.clone(),
        model,
    })
}

fn cape_from(texture: Texture) -> Cape {
    Cape {
        id: None,
        alias: None,
        url: texture.url,
        active: true,
    }
}

fn uploadable_texture(value: String) -> Option<AccountTextureType> {
    if value.eq_ignore_ascii_case("skin") {
        Some(AccountTextureType::Skin)
    } else if value.eq_ignore_ascii_case("cape") {
        Some(AccountTextureType::Cape)
    } else {
        None
    }
}
