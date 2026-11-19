use std::{fmt, path::PathBuf};

use elemental_core::auth::accounts::AccountId;
use elemental_shared::scope::Scope;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    texture::{TextureAsset, TextureId, TextureIndex, TextureSelection, TextureStore},
};

const CAPE_INDEX_VERSION: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapeId(u64);

impl fmt::Display for CapeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl TextureId for CapeId {
    fn from_u64(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapeAsset {
    pub id: CapeId,
    pub name: String,
}

impl TextureAsset for CapeAsset {
    type Id = CapeId;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn rename(&mut self, name: String) {
        self.name = name;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AccountCapeSelection {
    account_id: AccountId,
    cape_id: CapeId,
}

impl TextureSelection for AccountCapeSelection {
    type Id = CapeId;

    fn account_id(&self) -> AccountId {
        self.account_id
    }

    fn texture_id(&self) -> Self::Id {
        self.cape_id
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct CapeIndex {
    next_id: u64,
    capes: Vec<CapeAsset>,
    selections: Vec<AccountCapeSelection>,
}

impl TextureIndex for CapeIndex {
    type Id = CapeId;
    type Asset = CapeAsset;
    type Selection = AccountCapeSelection;

    fn parts(&self) -> (&u64, &Vec<Self::Asset>, &Vec<Self::Selection>) {
        (&self.next_id, &self.capes, &self.selections)
    }

    fn parts_mut(&mut self) -> (&mut u64, &mut Vec<Self::Asset>, &mut Vec<Self::Selection>) {
        (&mut self.next_id, &mut self.capes, &mut self.selections)
    }

    fn not_found(cape_id: CapeId) -> Error {
        Error::CapeNotFound { cape_id }
    }

    fn file_not_found(cape_id: CapeId, path: PathBuf) -> Error {
        Error::CapeFileNotFound { cape_id, path }
    }

    fn not_png(name: String) -> Error {
        Error::CapeNotPng { name }
    }

    fn id_exhausted() -> Error {
        Error::CapeIdExhausted
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedCape {
    pub asset: CapeAsset,
    pub path: PathBuf,
}

/// A locally stored cape and the path of its PNG file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapeEntry {
    pub asset: CapeAsset,
    pub path: PathBuf,
}

/// Stores cape PNG files locally and tracks per-account selections.
///
/// Selections are intended for offline profiles; online cape inventories and
/// selection remain part of the account API.
pub struct CapeStore {
    inner: TextureStore<CapeIndex>,
}

impl CapeStore {
    pub async fn open(scope: Scope) -> Result<Self> {
        Ok(Self {
            inner: TextureStore::open(scope, "capes", CAPE_INDEX_VERSION).await?,
        })
    }

    /// Imports PNG bytes into the local library.
    pub async fn import(&self, name: impl Into<String>, bytes: Vec<u8>) -> Result<CapeAsset> {
        self.inner
            .import(name, bytes, |id, name| CapeAsset { id, name })
            .await
    }

    /// Renames a local cape without changing its id, PNG file, or selections.
    pub async fn rename(&self, cape_id: CapeId, name: impl Into<String>) -> Result<()> {
        self.inner.rename(cape_id, name).await
    }

    /// Lists local capes with their PNG paths without reading the files.
    pub async fn list(&self) -> Result<Vec<CapeEntry>> {
        Ok(self
            .inner
            .list()
            .await?
            .into_iter()
            .map(|(asset, path)| CapeEntry { asset, path })
            .collect())
    }

    pub async fn remove(&self, cape_id: CapeId) -> Result<()> {
        self.inner.remove(cape_id).await
    }

    /// Selects a local cape for an account without uploading it.
    pub async fn select(&self, account_id: AccountId, cape_id: CapeId) -> Result<()> {
        self.inner
            .select(AccountCapeSelection {
                account_id,
                cape_id,
            })
            .await
    }

    pub async fn clear_selection(&self, account_id: AccountId) -> Result<()> {
        self.inner.clear_selection(account_id).await
    }

    pub async fn selected(&self, account_id: AccountId) -> Result<Option<SelectedCape>> {
        Ok(self
            .inner
            .selected(account_id)
            .await?
            .map(|(_, asset, path)| SelectedCape { asset, path }))
    }
}
