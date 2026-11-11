use elemental_core::auth::accounts::AccountId;
use elemental_shared::{
    migrator::NoMigrator,
    persistor::JsonFilePersistor,
    store::{Store, StoreLoader},
};
use serde::{Deserialize, Serialize};

pub(super) const ACCOUNT_INDEX_VERSION: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRef {
    pub account_id: AccountId,
    pub authorizer: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountIndex {
    pub selected: Option<AccountRef>,
    pub accounts: Vec<AccountRef>,
}

impl AccountIndex {
    pub(super) fn upsert(&mut self, reference: AccountRef) {
        match self
            .accounts
            .iter_mut()
            .find(|stored| stored.account_id == reference.account_id)
        {
            Some(stored) => *stored = reference.clone(),
            None => self.accounts.push(reference.clone()),
        }

        if self
            .selected
            .as_ref()
            .is_some_and(|selected| selected.account_id == reference.account_id)
        {
            self.selected = Some(reference);
        }
    }

    pub(super) fn remove(&mut self, account_id: AccountId) {
        self.accounts
            .retain(|stored| stored.account_id != account_id);
        self.selected = self
            .selected
            .take()
            .filter(|selected| selected.account_id != account_id);
    }
}

pub(super) type AccountIndexStore = StoreLoader<
    NoMigrator,
    Store<AccountIndex>,
    AccountIndex,
    JsonFilePersistor<Store<AccountIndex>>,
>;
