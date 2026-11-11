pub type DeviceCodeRequestError = oauth2::RequestTokenError<
    oauth2::HttpClientError<reqwest::Error>,
    oauth2::basic::BasicErrorResponse,
>;

pub type DeviceTokenRequestError = oauth2::RequestTokenError<
    oauth2::HttpClientError<reqwest::Error>,
    oauth2::DeviceCodeErrorResponse,
>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("build HTTP client failed")]
    HttpClientBuild {
        #[source]
        source: reqwest::Error,
    },

    #[error("parse {endpoint} endpoint failed")]
    Endpoint {
        endpoint: &'static str,
        #[source]
        source: oauth2::url::ParseError,
    },

    #[error("request device code failed")]
    DeviceCodeRequest {
        #[source]
        source: DeviceCodeRequestError,
    },

    #[error("request device access token failed")]
    DeviceTokenRequest {
        #[source]
        source: DeviceTokenRequestError,
    },

    #[error("refresh Microsoft access token failed")]
    RefreshTokenRequest {
        #[source]
        source: DeviceCodeRequestError,
    },

    #[error("exchange Microsoft token for Minecraft token failed")]
    MinecraftExchange {
        #[source]
        source: minecraft_msa_auth::MinecraftAuthorizationError<reqwest::Error>,
    },

    #[error("send Minecraft profile request failed")]
    ProfileRequest {
        #[source]
        source: reqwest::Error,
    },

    #[error("Minecraft profile request returned an unsuccessful status")]
    ProfileStatus {
        #[source]
        source: reqwest::Error,
    },

    #[error("Microsoft OAuth response contains no refresh token")]
    MicrosoftRefreshTokenMissing,

    #[error("request Xbox XUID failed")]
    XuidRequest {
        #[source]
        source: reqwest::Error,
    },

    #[error("decode Xbox XUID response failed")]
    XuidDecode {
        #[source]
        source: reqwest::Error,
    },

    #[error("Xbox XUID response contains no XUID")]
    XuidMissing,

    #[error("decode Minecraft profile response failed")]
    ProfileDecode {
        #[source]
        source: reqwest::Error,
    },

    #[error("texture operation '{operation}' is not supported for this account")]
    TextureUnsupported { operation: &'static str },

    #[error("send texture request for '{operation}' failed")]
    TextureRequest {
        operation: &'static str,
        #[source]
        source: reqwest::Error,
    },

    #[error("texture request for '{operation}' returned {status}: {body}")]
    TextureStatus {
        operation: &'static str,
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("unknown skin model '{model}'")]
    TextureModelUnknown { model: String },

    #[error("verification handler failed")]
    VerificationHandler {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[cfg(feature = "yggdrasil-auth")]
    #[error("Yggdrasil request failed")]
    Yggdrasil {
        #[source]
        source: nidhogg::error::NidhoggError,
    },

    #[cfg(feature = "yggdrasil-auth")]
    #[error("Yggdrasil authentication response contains no Minecraft profile")]
    YggdrasilProfileMissing,

    #[cfg(feature = "yggdrasil-auth")]
    #[error("parse Yggdrasil profile textures failed")]
    YggdrasilTexture {
        #[source]
        source: nidhogg::types::profile::ProfilePropertyError,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
