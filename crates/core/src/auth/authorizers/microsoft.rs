use crate::auth::accounts::microsoft::{MicrosoftAccount, unix_now};
use crate::auth::authorizers::Authorizer;
use crate::auth::error::{Error, Result, XuidDecodeError};
use crate::auth::texture::SkinModel;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use minecraft_msa_auth::MinecraftAuthorizationFlow;
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, ClientId, DeviceAuthorizationUrl, EndpointNotSet, EndpointSet, RefreshToken, Scope,
    StandardDeviceAuthorizationResponse, TokenResponse, TokenUrl,
};
use oauth2_reqwest::ReqwestClient;
use reqwest::Client;
use reqwest::ClientBuilder;
use reqwest::multipart::{Form, Part};
use reqwest::redirect::Policy;
use serde::Deserialize;

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const MSA_AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const MSA_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MINECRAFT_SKIN_UPLOAD_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const MINECRAFT_ACTIVE_SKIN_URL: &str =
    "https://api.minecraftservices.com/minecraft/profile/skins/active";
const MINECRAFT_ACTIVE_CAPE_URL: &str =
    "https://api.minecraftservices.com/minecraft/profile/capes/active";

pub const MICROSOFT_AUTHORIZER_ID: &str = "microsoft";

#[derive(Debug, Deserialize)]
pub(crate) struct MinecraftProfileResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) skins: Vec<MinecraftTexture>,
    #[serde(default)]
    pub(crate) capes: Vec<MinecraftTexture>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MinecraftTexture {
    pub(crate) id: Option<String>,
    pub(crate) state: String,
    pub(crate) url: String,
    pub(crate) variant: Option<String>,
    pub(crate) alias: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MinecraftAccessTokenClaims {
    xuid: String,
}

type RefreshClient =
    BasicClient<EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

/// Result returned by [`MicrosoftAuthorizer::verification_handler`].
pub type VerificationHandlerResult =
    std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub struct MicrosoftAuthorizer {
    pub client_id: String,
    /// Receives the verification URI and user code. Returning an error aborts authorization.
    pub verification_handler:
        Box<dyn Fn(String, String) -> VerificationHandlerResult + Send + Sync>,
}

impl MicrosoftAuthorizer {
    pub fn new(
        client_id: String,
        verification_handler: impl Fn(String, String) -> VerificationHandlerResult
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            client_id,
            verification_handler: Box::new(verification_handler),
        }
    }

    fn client() -> Result<ReqwestClient> {
        let reqwest_client = ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(Policy::none())
            .build()
            .map_err(|source| Error::HttpClientBuild { source })?;
        Ok(ReqwestClient::from(reqwest_client))
    }
}

impl Authorizer for MicrosoftAuthorizer {
    type Account = MicrosoftAccount;

    async fn authorize(&self) -> Result<Self::Account> {
        let reqwest_client = Self::client()?;
        let client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_auth_uri(auth_url()?)
            .set_token_uri(token_url()?)
            .set_device_authorization_url(device_authorization_url()?);

        let details: StandardDeviceAuthorizationResponse = client
            .exchange_device_code()
            .add_scope(Scope::new("XboxLive.signin offline_access".to_string()))
            .request_async(&reqwest_client)
            .await
            .map_err(|source| Error::DeviceCodeRequest { source })?;

        // Handle it outside to allow custom UX. A handler error aborts authorization.
        (self.verification_handler)(
            details.verification_uri().to_string(),
            details.user_code().secret().to_string(),
        )
        .map_err(|source| Error::VerificationHandler { source })?;

        let token = client
            .exchange_device_access_token(&details)
            .request_async(&reqwest_client, tokio::time::sleep, None)
            .await
            .map_err(|source| Error::DeviceTokenRequest { source })?;
        let refresh_token = token
            .refresh_token()
            .ok_or(Error::MicrosoftRefreshTokenMissing)?
            .secret()
            .to_owned();

        account_from_microsoft_token(
            &self.client_id,
            token.access_token().secret(),
            refresh_token,
            None,
        )
        .await
    }

    fn id() -> &'static str {
        MICROSOFT_AUTHORIZER_ID
    }

    fn name() -> &'static str {
        "Microsoft"
    }
}

pub(crate) async fn refresh(account: &MicrosoftAccount) -> Result<MicrosoftAccount> {
    let reqwest_client = MicrosoftAuthorizer::client()?;
    let client = refresh_client(&account.client_id)?;
    let token = client
        .exchange_refresh_token(&RefreshToken::new(account.refresh_token.clone()))
        .request_async(&reqwest_client)
        .await
        .map_err(|source| Error::RefreshTokenRequest { source })?;
    let refresh_token = token
        .refresh_token()
        .map(|token| token.secret().to_owned())
        .unwrap_or_else(|| account.refresh_token.clone());

    account_from_microsoft_token(
        &account.client_id,
        token.access_token().secret(),
        refresh_token,
        account.xuid.clone(),
    )
    .await
}

fn refresh_client(client_id: &str) -> Result<RefreshClient> {
    Ok(BasicClient::new(ClientId::new(client_id.to_owned())).set_token_uri(token_url()?))
}

fn auth_url() -> Result<AuthUrl> {
    AuthUrl::new(MSA_AUTHORIZE_URL.to_owned()).map_err(|source| Error::Endpoint {
        endpoint: "authorize",
        source,
    })
}

fn token_url() -> Result<TokenUrl> {
    TokenUrl::new(MSA_TOKEN_URL.to_owned()).map_err(|source| Error::Endpoint {
        endpoint: "token",
        source,
    })
}

fn device_authorization_url() -> Result<DeviceAuthorizationUrl> {
    DeviceAuthorizationUrl::new(DEVICE_CODE_URL.to_owned()).map_err(|source| Error::Endpoint {
        endpoint: "device authorization",
        source,
    })
}

async fn account_from_microsoft_token(
    client_id: &str,
    microsoft_access_token: &str,
    refresh_token: String,
    existing_xuid: Option<String>,
) -> Result<MicrosoftAccount> {
    let mc_flow = MinecraftAuthorizationFlow::new(Client::new());
    let mc_token = mc_flow
        .exchange_microsoft_token(microsoft_access_token)
        .await
        .map_err(|source| Error::MinecraftExchange { source })?;
    let profile = minecraft_profile(mc_token.access_token().as_ref()).await?;
    let xuid =
        existing_xuid.filter(|xuid| !xuid.is_empty()).or_else(|| {
            match xuid_from_access_token(mc_token.access_token().as_ref()) {
                Ok(xuid) => Some(xuid),
                Err(error) => {
                    tracing::warn!(%error, "failed to extract XUID from Minecraft access token");
                    None
                }
            }
        });

    Ok(MicrosoftAccount {
        account_id: crate::auth::accounts::AccountId::derive(MICROSOFT_AUTHORIZER_ID, &profile.id),
        username: profile.name,
        uuid: profile.id,
        access_token: mc_token.access_token().clone().into_inner(),
        refresh_token,
        client_id: client_id.to_owned(),
        xuid,
        expires_at: unix_now().saturating_add(u64::from(mc_token.expires_in())),
    })
}

pub(crate) async fn minecraft_profile(access_token: &str) -> Result<MinecraftProfileResponse> {
    Client::new()
        .get(MINECRAFT_PROFILE_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|source| Error::ProfileRequest { source })?
        .error_for_status()
        .map_err(|source| Error::ProfileStatus { source })?
        .json::<MinecraftProfileResponse>()
        .await
        .map_err(|source| Error::ProfileDecode { source })
}

pub(crate) async fn upload_skin(
    access_token: &str,
    image: Vec<u8>,
    model: SkinModel,
) -> Result<()> {
    let variant = match model {
        SkinModel::Classic => "classic",
        SkinModel::Slim => "slim",
    };
    let file = Part::bytes(image)
        .file_name("skin.png")
        .mime_str("image/png")
        .map_err(|source| Error::TextureRequest {
            operation: "upload skin",
            source,
        })?;
    let form = Form::new().text("variant", variant).part("file", file);
    send_texture_request(
        Client::new()
            .post(MINECRAFT_SKIN_UPLOAD_URL)
            .bearer_auth(access_token)
            .multipart(form),
        "upload skin",
    )
    .await
}

pub(crate) async fn delete_skin(access_token: &str) -> Result<()> {
    send_texture_request(
        Client::new()
            .delete(MINECRAFT_ACTIVE_SKIN_URL)
            .bearer_auth(access_token),
        "delete skin",
    )
    .await
}

pub(crate) async fn select_cape(access_token: &str, cape_id: Option<String>) -> Result<()> {
    let request = match cape_id {
        Some(cape_id) => Client::new()
            .put(MINECRAFT_ACTIVE_CAPE_URL)
            .json(&serde_json::json!({ "capeId": cape_id })),
        None => Client::new().delete(MINECRAFT_ACTIVE_CAPE_URL),
    };
    send_texture_request(request.bearer_auth(access_token), "select cape").await
}

async fn send_texture_request(
    request: reqwest::RequestBuilder,
    operation: &'static str,
) -> Result<()> {
    let response = request
        .send()
        .await
        .map_err(|source| Error::TextureRequest { operation, source })?;
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response
        .text()
        .await
        .map_err(|source| Error::TextureRequest { operation, source })?;
    Err(Error::TextureStatus {
        operation,
        status,
        body,
    })
}

fn xuid_from_access_token(access_token: &str) -> Result<String> {
    let mut parts = access_token.split('.');
    let (Some(_header), Some(payload), Some(_signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Error::XuidDecode {
            source: XuidDecodeError::MalformedToken,
        });
    };

    let payload = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|source| Error::XuidDecode {
            source: XuidDecodeError::Payload(source),
        })?;
    let claims: MinecraftAccessTokenClaims =
        serde_json::from_slice(&payload).map_err(|source| Error::XuidDecode {
            source: XuidDecodeError::Claims(source),
        })?;

    if claims.xuid.is_empty() {
        return Err(Error::XuidMissing);
    }
    Ok(claims.xuid)
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    use super::xuid_from_access_token;

    fn access_token(claims: &str) -> String {
        format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#),
            URL_SAFE_NO_PAD.encode(claims),
            URL_SAFE_NO_PAD.encode("signature"),
        )
    }

    #[test]
    fn decodes_xuid_from_minecraft_access_token() {
        let token = access_token(r#"{"xuid":"2535"}"#);
        assert_eq!(xuid_from_access_token(&token).unwrap(), "2535");
    }

    #[test]
    fn rejects_malformed_minecraft_access_token() {
        assert!(xuid_from_access_token("not-a-jwt").is_err());
    }
}
