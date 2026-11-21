#[cfg(feature = "microsoft-auth")]
pub type DeviceCodeRequestError = oauth2::RequestTokenError<
    oauth2::HttpClientError<reqwest::Error>,
    oauth2::basic::BasicErrorResponse,
>;

#[cfg(feature = "microsoft-auth")]
pub type DeviceTokenRequestError = oauth2::RequestTokenError<
    oauth2::HttpClientError<reqwest::Error>,
    oauth2::DeviceCodeErrorResponse,
>;

#[cfg(feature = "microsoft-auth")]
#[derive(Debug, thiserror::Error)]
pub enum XuidDecodeError {
    #[error("Minecraft access token is not a three-part JWT")]
    MalformedToken,

    #[error("decode Minecraft access token payload failed")]
    Payload(#[from] base64::DecodeError),

    #[error("parse Minecraft access token claims failed")]
    Claims(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[cfg(feature = "microsoft-auth")]
    #[error("build HTTP client failed")]
    HttpClientBuild {
        #[source]
        source: reqwest::Error,
    },

    #[cfg(feature = "microsoft-auth")]
    #[error("parse {endpoint} endpoint failed")]
    Endpoint {
        endpoint: &'static str,
        #[source]
        source: oauth2::url::ParseError,
    },

    #[cfg(feature = "microsoft-auth")]
    #[error("request device code failed")]
    DeviceCodeRequest {
        #[source]
        source: DeviceCodeRequestError,
    },

    #[cfg(feature = "microsoft-auth")]
    #[error("request device access token failed")]
    DeviceTokenRequest {
        #[source]
        source: DeviceTokenRequestError,
    },

    #[cfg(feature = "microsoft-auth")]
    #[error("refresh Microsoft access token failed")]
    RefreshTokenRequest {
        #[source]
        source: DeviceCodeRequestError,
    },

    #[cfg(feature = "microsoft-auth")]
    #[error("exchange Microsoft token for Minecraft token failed")]
    MinecraftExchange {
        #[source]
        source: minecraft_msa_auth::MinecraftAuthorizationError<reqwest::Error>,
    },

    #[cfg(feature = "microsoft-auth")]
    #[error("send Minecraft profile request failed")]
    ProfileRequest {
        #[source]
        source: reqwest::Error,
    },

    #[cfg(feature = "microsoft-auth")]
    #[error("Minecraft profile request returned an unsuccessful status")]
    ProfileStatus {
        #[source]
        source: reqwest::Error,
    },

    #[cfg(feature = "microsoft-auth")]
    #[error("Microsoft OAuth response contains no refresh token")]
    MicrosoftRefreshTokenMissing,

    #[cfg(feature = "microsoft-auth")]
    #[error("decode XUID from Minecraft access token failed")]
    XuidDecode {
        #[source]
        source: XuidDecodeError,
    },

    #[cfg(feature = "microsoft-auth")]
    #[error("Minecraft access token contains no XUID")]
    XuidMissing,

    #[cfg(feature = "microsoft-auth")]
    #[error("decode Minecraft profile response failed")]
    ProfileDecode {
        #[source]
        source: reqwest::Error,
    },

    #[error("texture operation '{operation}' is not supported for this account")]
    TextureUnsupported { operation: &'static str },

    #[cfg(feature = "microsoft-auth")]
    #[error("send texture request for '{operation}' failed")]
    TextureRequest {
        operation: &'static str,
        #[source]
        source: reqwest::Error,
    },

    #[cfg(feature = "microsoft-auth")]
    #[error("texture request for '{operation}' returned {status}: {body}")]
    TextureStatus {
        operation: &'static str,
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("unknown skin model '{model}'")]
    TextureModelUnknown { model: String },

    #[cfg(feature = "microsoft-auth")]
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
    #[error("Yggdrasil account has no base URL")]
    YggdrasilBaseUrlMissing,

    #[cfg(feature = "yggdrasil-auth")]
    #[error("parse Yggdrasil profile textures failed")]
    YggdrasilTexture {
        #[source]
        source: nidhogg::types::profile::ProfilePropertyError,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
