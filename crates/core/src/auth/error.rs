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

    #[error("decode Minecraft profile response failed")]
    ProfileDecode {
        #[source]
        source: reqwest::Error,
    },

    #[error("verification handler failed")]
    VerificationHandler {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
