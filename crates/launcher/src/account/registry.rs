use std::collections::HashMap;

use elemental_core::auth::{
    accounts::{Account, microsoft::MicrosoftAccount, offline::OfflineAccount},
    authorizers::{microsoft::MICROSOFT_AUTHORIZER_ID, offline::OFFLINE_AUTHORIZER_ID},
};
use serde::{Serialize, de::DeserializeOwned};

#[cfg(feature = "yggdrasil-auth")]
use elemental_core::auth::{
    accounts::yggdrasil::YggdrasilAccount, authorizers::yggdrasil::YGGDRASIL_AUTHORIZER_ID,
};

pub(super) type AccountDecoder =
    fn(&str) -> std::result::Result<Box<dyn Account>, serde_json::Error>;

/// Maps stable authorizer IDs to the account types they persist.
#[derive(Clone)]
pub struct AccountRegistry {
    decoders: HashMap<&'static str, AccountDecoder>,
}

impl AccountRegistry {
    pub fn new() -> Self {
        let registry = Self::empty()
            .with::<OfflineAccount>(OFFLINE_AUTHORIZER_ID)
            .with::<MicrosoftAccount>(MICROSOFT_AUTHORIZER_ID);
        #[cfg(feature = "yggdrasil-auth")]
        let registry = registry.with::<YggdrasilAccount>(YGGDRASIL_AUTHORIZER_ID);
        registry
    }

    pub fn empty() -> Self {
        Self {
            decoders: HashMap::new(),
        }
    }

    pub fn with<A>(mut self, authorizer_id: &'static str) -> Self
    where
        A: Account + DeserializeOwned + 'static,
    {
        self.register::<A>(authorizer_id);
        self
    }

    pub fn register<A>(&mut self, authorizer_id: &'static str)
    where
        A: Account + DeserializeOwned + 'static,
    {
        self.decoders.insert(authorizer_id, decode_account::<A>);
    }

    pub(super) fn decoder(&self, authorizer: &str) -> Option<AccountDecoder> {
        self.decoders.get(authorizer).copied()
    }
}

impl Default for AccountRegistry {
    fn default() -> Self {
        Self::new()
    }
}

struct SerializableAccount<'a>(&'a dyn Account);

impl Serialize for SerializableAccount<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        erased_serde::serialize(self.0, serializer)
    }
}

pub(super) fn encode_account(
    account: &dyn Account,
) -> std::result::Result<String, serde_json::Error> {
    serde_json::to_string(&SerializableAccount(account))
}

fn decode_account<A>(data: &str) -> std::result::Result<Box<dyn Account>, serde_json::Error>
where
    A: Account + DeserializeOwned + 'static,
{
    serde_json::from_str::<A>(data).map(|account| Box::new(account) as Box<dyn Account>)
}
