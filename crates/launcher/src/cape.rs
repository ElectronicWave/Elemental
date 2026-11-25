use std::{collections::HashSet, path::PathBuf};

use elemental_core::auth::accounts::AccountId;
use elemental_shared::{
    migrator::NoMigrator,
    persistor::{JsonFilePersistor, json_persistor},
    scope::Scope,
    store::{Store, StoreLoader},
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    texture::{PNG_SIGNATURE, TextureHash, gc_content, is_valid_name, remove_if_present},
};

pub use crate::texture::CapeId;

const CAPE_INDEX_VERSION: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapeAsset {
    /// SHA-256 content hash of the PNG bytes.
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
    capes: Vec<CapeAsset>,
    selections: Vec<AccountCapeSelection>,
}

impl CapeIndex {
    fn insert(&mut self, asset: CapeAsset) {
        if !self.capes.iter().any(|existing| existing.id == asset.id) {
            self.capes.push(asset);
        }
    }

    fn asset(&self, cape_id: CapeId) -> Option<CapeAsset> {
        self.capes.iter().find(|asset| asset.id == cape_id).cloned()
    }

    fn referenced_hashes(&self) -> HashSet<TextureHash> {
        self.capes.iter().map(|asset| asset.id.hash()).collect()
    }

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

/// Stores cape PNG files locally by SHA-256 content hash and tracks
/// per-account selections.
///
/// Selections are intended for offline profiles; online cape inventories and
/// selection remain part of the account API.
pub struct CapeStore {
    scope: Scope,
    index: CapeIndexStore,
}

impl CapeStore {
    pub async fn open(scope: Scope) -> Result<Self> {
        let index = Store::load(
            NoMigrator,
            json_persistor("capes/index".to_string(), scope.clone()),
            CAPE_INDEX_VERSION,
        )
        .await?;

        let store = Self { scope, index };
        store.gc().await?;
        Ok(store)
    }

    /// Imports PNG bytes into the local library.
    pub async fn import(&self, name: impl Into<String>, bytes: Vec<u8>) -> Result<CapeAsset> {
        let name = name.into();
        if !is_valid_name(&name) {
            return Err(Error::TextureNameEmpty);
        }
        if !bytes.starts_with(PNG_SIGNATURE) {
            return Err(Error::CapeNotPng { name });
        }

        let hash = TextureHash::of(&bytes);
        let id = CapeId::from_hash(hash);
        let path = self.path(id).await?;
        tokio::fs::write(&path, &bytes).await?;

        let inserted = CapeAsset { id, name };

        self.index.set(move |index| index.insert(inserted)).await?;
        self.asset(id)
            .await
            .ok_or(Error::CapeNotFound { cape_id: id })
    }

    /// Renames a local cape without changing its id, PNG file, or selections.
    pub async fn rename(&self, cape_id: CapeId, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        if !is_valid_name(&name) {
            return Err(Error::TextureNameEmpty);
        }
        if self.asset(cape_id).await.is_none() {
            return Err(Error::CapeNotFound { cape_id });
        }

        self.index.set(|index| index.rename(cape_id, name)).await?;
        Ok(())
    }

    /// Lists local capes with their PNG paths without reading the files.
    pub async fn list(&self) -> Result<Vec<CapeEntry>> {
        let assets = self.index.get(|index| index.capes.clone()).await;
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
        self.index.set(|index| index.remove(cape_id)).await?;
        remove_if_present(&path).await?;
        Ok(())
    }

    /// Removes unreferenced content files left behind by interrupted imports.
    async fn gc(&self) -> Result<()> {
        let referenced = self.index.get(CapeIndex::referenced_hashes).await;
        let root_path = self.scope.get_full_path("capes", None).await?;
        gc_content(&root_path, &referenced).await?;
        Ok(())
    }

    /// Selects a local cape for an account without uploading it.
    pub async fn select(&self, account_id: AccountId, cape_id: CapeId) -> Result<()> {
        if self.asset(cape_id).await.is_none() {
            return Err(Error::CapeNotFound { cape_id });
        }

        self.index
            .set(|index| {
                index.select(AccountCapeSelection {
                    account_id,
                    cape_id,
                });
            })
            .await?;
        Ok(())
    }

    pub async fn clear_selection(&self, account_id: AccountId) -> Result<()> {
        self.index
            .set(|index| index.clear_selection(account_id))
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
        self.index.get(|index| index.asset(cape_id)).await
    }

    async fn selection(&self, account_id: AccountId) -> Option<AccountCapeSelection> {
        self.index
            .get(|index| index.selected_by(account_id).cloned())
            .await
    }

    async fn path(&self, cape_id: CapeId) -> Result<PathBuf> {
        let hash = cape_id.hash().hex();
        let prefix = &hash[..2];
        Ok(self
            .scope
            .get_full_path(&format!("capes/{prefix}/{hash}"), Some("png".to_string()))
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
