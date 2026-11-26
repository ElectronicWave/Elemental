use crate::auth::accounts::AccountId;
pub use crate::auth::accounts::yggdrasil::YGGDRASIL_AUTHORIZER_ID;
use crate::auth::accounts::yggdrasil::{
    YggdrasilAccount, YggdrasilProfile, preferred_language, user_id,
};
use crate::auth::authorizers::Authorizer;
use crate::auth::error::{Error, Result};
use nidhogg::{client::Nidhogg, request::authenticate::Authenticate};

pub struct YggdrasilAuthorizer {
    pub base_url: Option<String>,
    pub username: String,
    pub password: String,
    pub client_token: Option<String>,
}

impl YggdrasilAuthorizer {
    fn client(&self) -> Result<Nidhogg> {
        let base_url = self
            .base_url
            .as_deref()
            .ok_or(Error::YggdrasilBaseUrlMissing)?;
        Self::client_for(base_url)
    }

    fn client_for(base_url: &str) -> Result<Nidhogg> {
        Nidhogg::new(base_url.trim_end_matches('/')).map_err(|source| Error::Yggdrasil { source })
    }

    pub async fn authenticate(&self) -> Result<YggdrasilAccount> {
        let Some(base_url) = self.base_url.as_deref() else {
            return Ok(YggdrasilAccount::offline(self.username.clone()));
        };
        let base_url = normalized_base_url(base_url);
        let response = self
            .client()?
            .authenticate(&Authenticate::new(
                self.username.clone(),
                self.password.clone(),
                self.client_token.clone(),
                true,
            ))
            .await
            .map_err(|source| Error::Yggdrasil { source })?;
        let selected_profile = response
            .selected_profile
            .as_ref()
            .or_else(|| {
                response
                    .available_profiles
                    .as_ref()
                    .and_then(|profiles| profiles.first())
            })
            .ok_or(Error::YggdrasilProfileMissing)?;

        Ok(YggdrasilAccount {
            base_url: Some(base_url.to_owned()),
            account_id: account_id(base_url, &selected_profile.id),
            user_id: user_id(response.user.as_ref()),
            preferred_language: preferred_language(response.user.as_ref()),
            access_token: response.access_token,
            client_token: response.client_token,
            selected_profile: YggdrasilProfile::from(selected_profile),
        })
    }

    pub async fn refresh_account(account: &YggdrasilAccount) -> Result<YggdrasilAccount> {
        account.refresh_account().await
    }

    pub async fn refresh(&self, account: &YggdrasilAccount) -> Result<YggdrasilAccount> {
        Self::refresh_account(account).await
    }
}

fn normalized_base_url(base_url: &str) -> &str {
    base_url.trim_end_matches('/')
}

fn account_id(base_url: &str, profile_id: &str) -> AccountId {
    AccountId::derive(YGGDRASIL_AUTHORIZER_ID, &format!("{base_url}:{profile_id}"))
}

impl Authorizer for YggdrasilAuthorizer {
    type Account = YggdrasilAccount;

    async fn authorize(&self) -> Result<Self::Account> {
        self.authenticate().await
    }

    fn id() -> &'static str {
        YGGDRASIL_AUTHORIZER_ID
    }

    fn name() -> &'static str {
        "Yggdrasil"
    }
}
