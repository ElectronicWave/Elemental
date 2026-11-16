use std::{io::ErrorKind, path::PathBuf};

use elemental_core::auth::{
    accounts::{Account, AccountId},
    authorizers::Authorizer,
};
use elemental_shared::{
    migrator::NoMigrator, persistor::json_persistor, scope::Scope, store::Store,
};

use super::{
    index::{ACCOUNT_INDEX_VERSION, AccountIndex, AccountIndexStore, AccountRef},
    registry::{AccountDecoder, AccountRegistry, encode_account},
};
use crate::error::{Error, Result};

pub struct AccountStore {
    scope: Scope,
    index: AccountIndexStore,
    registry: AccountRegistry,
}

impl AccountStore {
    pub async fn open(scope: Scope) -> Result<Self> {
        Self::with_registry(scope, AccountRegistry::new()).await
    }

    pub async fn with_registry(scope: Scope, registry: AccountRegistry) -> Result<Self> {
        let index = Store::load(
            NoMigrator,
            json_persistor("accounts/index".to_owned(), scope.clone()),
            ACCOUNT_INDEX_VERSION,
        )
        .await?;

        Ok(Self {
            scope,
            index,
            registry,
        })
    }

    pub async fn index(&self) -> AccountIndex {
        self.index.cloned().await.value
    }

    fn decoder(&self, reference: &AccountRef) -> Result<AccountDecoder> {
        self.registry
            .decoder(&reference.authorizer)
            .ok_or_else(|| Error::UnknownAuthorizer {
                account_id: reference.account_id,
                authorizer: reference.authorizer.clone(),
            })
    }

    pub async fn save(&self, account: &dyn Account) -> Result<()> {
        let account_id = account.account_id();
        let reference = AccountRef {
            account_id,
            authorizer: account.authorizer_id().to_owned(),
        };
        self.decoder(&reference)?;
        let data = encode_account(account).map_err(|source| Error::AccountEncode {
            account_id: reference.account_id,
            authorizer: reference.authorizer.clone(),
            source,
        })?;
        let path = self.account_path(&reference).await?;
        tokio::fs::write(path, data).await?;
        self.index
            .set(|index| index.value.upsert(reference))
            .await?;
        Ok(())
    }

    /// Runs an authorizer and persists the resulting account.
    ///
    /// Selecting the account is left to the caller so a failed sign-in never
    /// changes the currently selected account.
    pub async fn sign_in<A>(&self, authorizer: &A) -> Result<A::Account>
    where
        A: Authorizer,
    {
        let account = authorizer.authorize().await?;
        self.save(&account).await?;
        Ok(account)
    }

    pub async fn remove(&self, account_id: AccountId) -> Result<()> {
        let reference = self.reference(account_id).await?;
        let path = self.account_path(&reference).await?;
        tokio::fs::remove_file(path).await?;

        self.index
            .set(|index| index.value.remove(account_id))
            .await?;
        Ok(())
    }

    pub async fn load(&self, reference: &AccountRef) -> Result<Box<dyn Account>> {
        let decoder = self.decoder(reference)?;
        let path = self.account_path(reference).await?;
        let data = match tokio::fs::read_to_string(path).await {
            Ok(data) => data,
            Err(source) if source.kind() == ErrorKind::NotFound => {
                return Err(Error::AccountNotFound {
                    account_id: reference.account_id,
                });
            }
            Err(source) => return Err(source.into()),
        };
        let account = decoder(&data).map_err(|source| Error::AccountDecode {
            account_id: reference.account_id,
            authorizer: reference.authorizer.clone(),
            source,
        })?;

        if account.account_id() != reference.account_id
            || account.authorizer_id() != reference.authorizer.as_str()
        {
            return Err(Error::AccountMismatch {
                expected_id: reference.account_id,
                expected_authorizer: reference.authorizer.clone(),
                actual_id: account.account_id(),
                actual_authorizer: account.authorizer_id().to_owned(),
            });
        }

        Ok(account)
    }

    pub async fn list(&self) -> Result<Vec<Box<dyn Account>>> {
        let references = self.index.get(|index| index.value.accounts.clone()).await;
        let mut accounts = Vec::with_capacity(references.len());
        for reference in references {
            accounts.push(self.load(&reference).await?);
        }
        Ok(accounts)
    }

    pub async fn selected(&self) -> Result<Option<Box<dyn Account>>> {
        let reference = self.index.get(|index| index.value.selected.clone()).await;
        match reference {
            Some(reference) => Ok(Some(self.load(&reference).await?)),
            None => Ok(None),
        }
    }

    pub async fn refresh(&self, account_id: AccountId) -> Result<Box<dyn Account>> {
        let account = self.load_by_id(account_id).await?;
        self.persist_refresh(account).await
    }

    pub async fn select(&self, account_id: AccountId) -> Result<()> {
        let reference = self.reference(account_id).await?;

        self.index
            .set(|index| index.value.selected = Some(reference))
            .await?;
        Ok(())
    }

    /// Loads an account and refreshes it first when its credential is expiring.
    ///
    /// The refreshed account is persisted before it is returned.
    pub async fn account(&self, account_id: AccountId) -> Result<Box<dyn Account>> {
        let account = self.load_by_id(account_id).await?;
        if account.needs_refresh() {
            self.persist_refresh(account).await
        } else {
            Ok(account)
        }
    }

    async fn load_by_id(&self, account_id: AccountId) -> Result<Box<dyn Account>> {
        let reference = self.reference(account_id).await?;
        self.load(&reference).await
    }

    async fn persist_refresh(&self, account: Box<dyn Account>) -> Result<Box<dyn Account>> {
        let refreshed = account.refresh().await?;
        self.save(refreshed.as_ref()).await?;
        Ok(refreshed)
    }

    async fn reference(&self, account_id: AccountId) -> Result<AccountRef> {
        self.index
            .get(|index| {
                index
                    .value
                    .accounts
                    .iter()
                    .find(|stored| stored.account_id == account_id)
                    .cloned()
            })
            .await
            .ok_or(Error::AccountNotFound { account_id })
    }

    async fn account_path(&self, reference: &AccountRef) -> Result<PathBuf> {
        let id = format!("accounts/{}/{}", reference.authorizer, reference.account_id);
        Ok(self
            .scope
            .get_full_path(&id, Some("json".to_owned()))
            .await?)
    }
}
