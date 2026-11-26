use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use minecraft_msa_auth::MinecraftAuthorizationFlow;
use oauth2::basic::BasicClient;
use oauth2::{ClientId, EndpointNotSet, EndpointSet, RefreshToken, TokenResponse, TokenUrl};
use oauth2_reqwest::ReqwestClient;
use reqwest::multipart::{Form, Part};
use reqwest::redirect::Policy;
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;

use crate::auth::error::{Error, Result, XuidDecodeError};
use crate::auth::texture::{Cape, ProfileTextures, Skin, SkinModel, TextureType};

const MSA_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MINECRAFT_SKIN_UPLOAD_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const MINECRAFT_ACTIVE_SKIN_URL: &str =
    "https://api.minecraftservices.com/minecraft/profile/skins/active";
const MINECRAFT_ACTIVE_CAPE_URL: &str =
    "https://api.minecraftservices.com/minecraft/profile/capes/active";

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

pub(crate) struct ExchangedMinecraftSession {
    pub(crate) username: String,
    pub(crate) uuid: String,
    pub(crate) access_token: String,
    pub(crate) refresh_token: String,
    pub(crate) xuid: Option<String>,
    pub(crate) expires_at: u64,
}

type RefreshClient =
    BasicClient<EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

pub(crate) fn oauth_http_client() -> Result<ReqwestClient> {
    let reqwest_client = ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(Policy::none())
        .build()
        .map_err(|source| Error::HttpClientBuild { source })?;
    Ok(ReqwestClient::from(reqwest_client))
}

pub(crate) fn token_url() -> Result<TokenUrl> {
    TokenUrl::new(MSA_TOKEN_URL.to_owned()).map_err(|source| Error::Endpoint {
        endpoint: "token",
        source,
    })
}

pub(crate) async fn refresh_microsoft_session(
    client_id: &str,
    existing_refresh_token: &str,
    existing_xuid: Option<String>,
) -> Result<ExchangedMinecraftSession> {
    let reqwest_client = oauth_http_client()?;
    let client = refresh_client(client_id)?;
    let token = client
        .exchange_refresh_token(&RefreshToken::new(existing_refresh_token.to_owned()))
        .request_async(&reqwest_client)
        .await
        .map_err(|source| Error::RefreshTokenRequest { source })?;
    let refresh_token = token
        .refresh_token()
        .map(|token| token.secret().to_owned())
        .unwrap_or_else(|| existing_refresh_token.to_owned());

    exchange_microsoft_token(token.access_token().secret(), refresh_token, existing_xuid).await
}

fn refresh_client(client_id: &str) -> Result<RefreshClient> {
    Ok(BasicClient::new(ClientId::new(client_id.to_owned())).set_token_uri(token_url()?))
}

pub(crate) async fn exchange_microsoft_token(
    microsoft_access_token: &str,
    refresh_token: String,
    existing_xuid: Option<String>,
) -> Result<ExchangedMinecraftSession> {
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

    Ok(ExchangedMinecraftSession {
        username: profile.name,
        uuid: profile.id,
        access_token: mc_token.access_token().clone().into_inner(),
        refresh_token,
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

    let bytes = response
        .bytes()
        .await
        .map_err(|source| Error::TextureRequest { operation, source })?;
    let body = String::from_utf8_lossy(&bytes).into_owned();
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

pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

pub(crate) fn profile_textures(profile: MinecraftProfileResponse) -> Result<ProfileTextures> {
    Ok(ProfileTextures {
        skin: active_skin(profile.skins)?,
        capes: profile.capes.into_iter().map(cape_from).collect(),
        uploadable: vec![TextureType::Skin],
    })
}

fn active_skin(skins: Vec<MinecraftTexture>) -> Result<Option<Skin>> {
    skins
        .into_iter()
        .find(|texture| texture.state.eq_ignore_ascii_case("ACTIVE"))
        .map(skin_from)
        .transpose()
}

fn skin_from(texture: MinecraftTexture) -> Result<Skin> {
    let variant = texture.variant.unwrap_or_default();
    let model = match variant.to_ascii_lowercase().as_str() {
        "" | "classic" => SkinModel::Classic,
        "slim" => SkinModel::Slim,
        _ => return Err(Error::TextureModelUnknown { model: variant }),
    };
    Ok(Skin {
        url: texture.url,
        model,
    })
}

fn cape_from(texture: MinecraftTexture) -> Cape {
    Cape {
        id: texture.id,
        alias: texture.alias,
        url: texture.url,
        active: texture.state.eq_ignore_ascii_case("ACTIVE"),
    }
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
