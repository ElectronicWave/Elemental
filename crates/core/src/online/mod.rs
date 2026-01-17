use oauth2::basic::{BasicClient, BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse, BasicTokenResponse};
use oauth2::{AuthUrl, Client, ClientId, DeviceAuthorizationUrl, EndpointNotSet, EndpointSet, Scope, StandardDeviceAuthorizationResponse, StandardRevocableToken, TokenResponse, TokenUrl};
use anyhow::Result;
use minecraft_msa_auth::{MinecraftAuthenticationResponse, MinecraftAuthorizationFlow};

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const MSA_AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const MSA_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";

pub type OAuthClient = Client<
    BasicErrorResponse,
    BasicTokenResponse,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    EndpointSet,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet
>;

pub fn create_oauth_client(client_id: &str) -> OAuthClient
{
    BasicClient::new(ClientId::new(client_id.to_string()))
        .set_auth_uri(AuthUrl::new(MSA_AUTHORIZE_URL.to_string()).unwrap())
        .set_token_uri(TokenUrl::new(MSA_TOKEN_URL.to_string()).unwrap())
        .set_device_authorization_url(DeviceAuthorizationUrl::new(DEVICE_CODE_URL.to_string()).unwrap())
}

pub struct Authorization {
    pub client: OAuthClient
}

impl Authorization {
    pub fn new(client_id: &str) -> Result<Self> {
        Ok(Self { client: create_oauth_client(client_id) })
    }

    pub fn custom(client: OAuthClient) -> Result<Self> {
        Ok(Self { client })
    }

    pub async fn verify(&self) -> Verification {
        let details: StandardDeviceAuthorizationResponse = self.client
            .exchange_device_code()
            .add_scope(Scope::new("XboxLive.signin offline_access".to_string()))
            .request_async(&reqwest::Client::new()).await.unwrap();

        Verification { 0: details.device_code().secret().to_string(), 1: details.verification_uri().to_string() }
    }

    pub async fn token(&self) -> BasicTokenResponse {
        let details: StandardDeviceAuthorizationResponse = self.client
            .exchange_device_code()
            .add_scope(Scope::new("XboxLive.signin offline_access".to_string()))
            .request_async(&reqwest::Client::new()).await.unwrap();

        self.client.exchange_device_access_token(&details).request_async(&reqwest::Client::new(), tokio::time::sleep, None).await.unwrap()
    }

    pub async fn mc_token(&self, token: BasicTokenResponse) -> MinecraftAuthenticationResponse {
        let mc_flow = MinecraftAuthorizationFlow::new(reqwest::Client::new());
        mc_flow.exchange_microsoft_token(token.access_token().secret()).await.unwrap()
    }
}

pub struct Verification(String, String);