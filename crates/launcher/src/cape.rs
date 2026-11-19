use std::{fmt, path::PathBuf};

use elemental_core::auth::accounts::AccountId;
use elemental_shared::{
    migrator::NoMigrator,
    persistor::{JsonFilePersistor, json_persistor},
    scope::Scope,
    store::{Store, StoreLoader},
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    error::{Error, Result},
    texture::{PNG_SIGNATURE, validate_name},
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapeAsset {
    pub id: CapeId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AccountCapeSelection {
    account_id: AccountId,
    cape_id: CapeId,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct CapeIndex {
    next_id: u64,
    capes: Vec<CapeAsset>,
    selections: Vec<AccountCapeSelection>,
}

impl CapeIndex {
    fn remove(&mut self, cape_id: CapeId) {
        self.capes.retain(|asset| asset.id != cape_id);
        self.selections
            .retain(|selection| selection.cape_id != cape_id);
    }

    fn selected_by(&self, account_id: AccountId) -> Option<&AccountCapeSelection> {
        self.selections
            .iter()
            .find(|selection| selection.account_id == account_id)
    }

    fn select(&mut self, selection: AccountCapeSelection) {
        self.clear_selection(selection.account_id);
        self.selections.push(selection);
    }

    fn clear_selection(&mut self, account_id: AccountId) {
        self.selections
            .retain(|selection| selection.account_id != account_id);
    }

    fn rename(&mut self, cape_id: CapeId, name: String) {
        if let Some(asset) = self.capes.iter_mut().find(|asset| asset.id == cape_id) {
            asset.name = name;
        }
    }
}

type CapeIndexStore =
    StoreLoader<NoMigrator, Store<CapeIndex>, CapeIndex, JsonFilePersistor<Store<CapeIndex>>>;

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
    scope: Scope,
    index: CapeIndexStore,
    import_lock: Mutex<()>,
}

impl CapeStore {
    pub async fn open(scope: Scope) -> Result<Self> {
        let index = Store::load(
            NoMigrator,
            json_persistor("capes/index".to_string(), scope.clone()),
            CAPE_INDEX_VERSION,
        )
        .await?;

        Ok(Self {
            scope,
            index,
            import_lock: Mutex::new(()),
        })
    }

    /// Imports PNG bytes into the local library.
    pub async fn import(&self, name: impl Into<String>, bytes: Vec<u8>) -> Result<CapeAsset> {
        let name = name.into();
        validate_name(&name)?;
        if !bytes.starts_with(PNG_SIGNATURE) {
            return Err(Error::CapeNotPng { name });
        }

        let _guard = self.import_lock.lock().await;
        let next_id = self
            .index
            .get(|index| index.value.next_id.checked_add(1))
            .await
            .ok_or(Error::CapeIdExhausted)?;
        let asset = CapeAsset {
            id: CapeId(next_id),
            name,
        };

        let path = self.path(asset.id).await?;
        tokio::fs::write(&path, bytes).await?;

        self.index
            .set(|index| {
                index.value.next_id = next_id;
                index.value.capes.push(asset.clone());
            })
            .await?;
        Ok(asset)
    }

    /// Renames a local cape without changing its id, PNG file, or selections.
    pub async fn rename(&self, cape_id: CapeId, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        validate_name(&name)?;
        if self.asset(cape_id).await.is_none() {
            return Err(Error::CapeNotFound { cape_id });
        }

        self.index
            .set(|index| index.value.rename(cape_id, name))
            .await?;
        Ok(())
    }

    /// Lists local capes with their PNG paths without reading the files.
    pub async fn list(&self) -> Result<Vec<CapeEntry>> {
        let assets = self.index.get(|index| index.value.capes.clone()).await;
        let mut entries = Vec::with_capacity(assets.len());
        for asset in assets {
            let path = self.path(asset.id).await?;
            entries.push(CapeEntry { asset, path });
        }
        Ok(entries)
    }

    pub async fn remove(&self, cape_id: CapeId) -> Result<()> {
        let asset = self
            .asset(cape_id)
            .await
            .ok_or(Error::CapeNotFound { cape_id })?;

        let path = self.path(asset.id).await?;
        self.index.set(|index| index.value.remove(cape_id)).await?;
        tokio::fs::remove_file(path).await?;
        Ok(())
    }

    /// Selects a local cape for an account without uploading it.
    pub async fn select(&self, account_id: AccountId, cape_id: CapeId) -> Result<()> {
        if self.asset(cape_id).await.is_none() {
            return Err(Error::CapeNotFound { cape_id });
        }

        self.index
            .set(|index| {
                index.value.select(AccountCapeSelection {
                    account_id,
                    cape_id,
                });
            })
            .await?;
        Ok(())
    }

    pub async fn clear_selection(&self, account_id: AccountId) -> Result<()> {
        self.index
            .set(|index| index.value.clear_selection(account_id))
            .await?;
        Ok(())
    }

    pub async fn selected(&self, account_id: AccountId) -> Result<Option<SelectedCape>> {
        let Some(selection) = self.selection(account_id).await else {
            return Ok(None);
        };
        let (asset, path) = self.existing(selection.cape_id).await?;

        Ok(Some(SelectedCape { asset, path }))
    }

    async fn asset(&self, cape_id: CapeId) -> Option<CapeAsset> {
        self.index
            .get(|index| {
                index
                    .value
                    .capes
                    .iter()
                    .find(|asset| asset.id == cape_id)
                    .cloned()
            })
            .await
    }

    async fn selection(&self, account_id: AccountId) -> Option<AccountCapeSelection> {
        self.index
            .get(|index| index.value.selected_by(account_id).cloned())
            .await
    }

    async fn path(&self, cape_id: CapeId) -> Result<PathBuf> {
        Ok(self
            .scope
            .get_full_path(&format!("capes/files/{cape_id}"), Some("png".to_string()))
            .await?)
    }

    async fn existing(&self, cape_id: CapeId) -> Result<(CapeAsset, PathBuf)> {
        let asset = self
            .asset(cape_id)
            .await
            .ok_or(Error::CapeNotFound { cape_id })?;
        let path = self.path(cape_id).await?;
        if !tokio::fs::try_exists(&path).await? {
            return Err(Error::CapeFileNotFound { cape_id, path });
        }
        Ok((asset, path))
    }
}
