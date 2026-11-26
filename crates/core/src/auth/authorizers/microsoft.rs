pub use crate::auth::accounts::microsoft::MICROSOFT_AUTHORIZER_ID;
use crate::auth::accounts::microsoft::MicrosoftAccount;
use crate::auth::authorizers::Authorizer;
use crate::auth::error::{Error, Result};
use crate::auth::microsoft_protocol::{exchange_microsoft_token, oauth_http_client, token_url};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, ClientId, DeviceAuthorizationUrl, Scope, StandardDeviceAuthorizationResponse,
    TokenResponse,
};

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const MSA_AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";

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
}

impl Authorizer for MicrosoftAuthorizer {
    type Account = MicrosoftAccount;

    async fn authorize(&self) -> Result<Self::Account> {
        let reqwest_client = oauth_http_client()?;
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

        let session =
            exchange_microsoft_token(token.access_token().secret(), refresh_token, None).await?;

        Ok(MicrosoftAccount::from_session(&self.client_id, session))
    }

    fn id() -> &'static str {
        MICROSOFT_AUTHORIZER_ID
    }

    fn name() -> &'static str {
        "Microsoft"
    }
}

fn auth_url() -> Result<AuthUrl> {
    AuthUrl::new(MSA_AUTHORIZE_URL.to_owned()).map_err(|source| Error::Endpoint {
        endpoint: "authorize",
        source,
    })
}

fn device_authorization_url() -> Result<DeviceAuthorizationUrl> {
    DeviceAuthorizationUrl::new(DEVICE_CODE_URL.to_owned()).map_err(|source| Error::Endpoint {
        endpoint: "device authorization",
        source,
    })
}
