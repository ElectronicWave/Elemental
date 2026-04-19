use crate::auth::authorizer::Authorizer;
use crate::auth::credential::UserCredential;
use anyhow::{Context, Result};
use minecraft_msa_auth::MinecraftAuthorizationFlow;
use oauth2::basic::BasicClient;
use oauth2::reqwest::Client;
use oauth2::{
    AuthUrl, ClientId, DeviceAuthorizationUrl, Scope, StandardDeviceAuthorizationResponse,
    TokenResponse, TokenUrl,
};

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const MSA_AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const MSA_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(serde::Deserialize)]
struct MinecraftProfileResponse {
    id: String,
    name: String,
}

pub struct MicrosoftAuthorizer<F: Fn(String, String)> {
    pub client_id: String,
    pub verification_handler: F,
}

impl<F: Fn(String, String)> Authorizer for MicrosoftAuthorizer<F> {
    async fn authorize(&self) -> Result<UserCredential> {
        let request_client = Client::new();
        let client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_auth_uri(AuthUrl::new(MSA_AUTHORIZE_URL.to_string())?)
            .set_token_uri(TokenUrl::new(MSA_TOKEN_URL.to_string())?)
            .set_device_authorization_url(DeviceAuthorizationUrl::new(
                DEVICE_CODE_URL.to_string(),
            )?);

        let details: StandardDeviceAuthorizationResponse = client
            .exchange_device_code()
            .add_scope(Scope::new("XboxLive.signin offline_access".to_string()))
            .request_async(&request_client)
            .await?;

        // Handle it outside to allow custom UX.
        (self.verification_handler)(
            details.verification_uri().to_string(),
            details.user_code().secret().to_string(),
        );

        let token = client
            .exchange_device_access_token(&details)
            .request_async(&request_client, tokio::time::sleep, None)
            .await?;

        let mc_flow = MinecraftAuthorizationFlow::new(Client::new());
        let mc_token = mc_flow
            .exchange_microsoft_token(token.access_token().secret())
            .await?;
        let profile = reqwest::Client::new()
            .get(MINECRAFT_PROFILE_URL)
            .bearer_auth(mc_token.access_token().as_ref())
            .send()
            .await?
            .error_for_status()?
            .json::<MinecraftProfileResponse>()
            .await
            .context("fetch minecraft profile failed")?;

        Ok(UserCredential {
            username: profile.name,
            uuid: profile.id,
            access_token: mc_token.access_token().clone().into_inner(),
        })
    }

    fn name() -> &'static str {
        "Microsoft"
    }
}
