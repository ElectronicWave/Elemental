pub mod microsoft;
pub mod offline;
#[cfg(feature = "yggdrasil-auth")]
pub mod yggdrasil;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::credential::Credential;
use crate::auth::error::Result;
use crate::auth::texture::{ProfileTextures, SkinModel};

fn unsupported<T>(operation: &'static str) -> Result<T> {
    Err(crate::auth::error::Error::TextureUnsupported { operation })
}

/// Stable identity of a persisted account.
///
/// This is independent from the Minecraft profile UUID because Yggdrasil
/// profiles from different API roots may share the same profile ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountId(Uuid);

impl AccountId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Derives a stable account identity from its authorizer and provider-specific identity.
    pub fn derive(authorizer_id: &str, identity: &str) -> Self {
        Self(Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!("elemental:{authorizer_id}:{identity}").as_bytes(),
        ))
    }
}

impl Default for AccountId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Persisted authentication state owned by one [`crate::auth::authorizers::Authorizer`].
#[async_trait]
pub trait Account: Send + Sync + erased_serde::Serialize {
    fn account_id(&self) -> AccountId;

    fn authorizer_id(&self) -> &'static str;

    fn display_name(&self) -> &str;

    fn needs_refresh(&self) -> bool {
        false
    }

    async fn credential(&self) -> Result<Credential>;

    async fn refresh(&self) -> Result<Box<dyn Account>>;

    async fn textures(&self) -> Result<ProfileTextures> {
        unsupported("read textures")
    }

    async fn upload_skin(&self, _image: Vec<u8>, _model: SkinModel) -> Result<()> {
        unsupported("upload skin")
    }

    async fn delete_skin(&self) -> Result<()> {
        unsupported("delete skin")
    }

    async fn upload_cape(&self, _image: Vec<u8>) -> Result<()> {
        unsupported("upload cape")
    }

    async fn delete_cape(&self) -> Result<()> {
        unsupported("delete cape")
    }

    /// Selects an owned cape, or clears the active cape when `cape_id` is `None`.
    async fn select_cape(&self, _cape_id: Option<String>) -> Result<()> {
        unsupported("select cape")
    }
}
