use serde::{Deserialize, Serialize};

/// Credential passed to the game process for one launch.
///
/// This value is derived from an [`crate::auth::accounts::Account`] and is
/// intentionally not persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credential {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub kind: CredentialKind,
    pub xuid: Option<String>,
    pub client_id: Option<String>,
    /// Yggdrasil API root for authlib-injector.
    #[cfg(feature = "yggdrasil-auth")]
    pub auth_server: Option<String>,
}

impl Credential {
    #[cfg(feature = "yggdrasil-auth")]
    pub fn with_auth_server(mut self, auth_server: impl Into<String>) -> Self {
        self.auth_server = Some(auth_server.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialKind {
    Legacy,
    Mojang,
    Msa,
}
