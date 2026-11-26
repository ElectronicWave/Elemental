use std::{collections::HashSet, future::Future, path::PathBuf};

use elemental_core::auth::{
    accounts::{Account, AccountId},
    texture::SkinModel,
};
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

pub use crate::texture::SkinId;

const SKIN_INDEX_VERSION: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkinAsset {
    /// SHA-256 content hash of the PNG bytes.
    pub id: SkinId,
    pub name: String,
    pub default_model: SkinModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AccountSkinSelection {
    account_id: AccountId,
    skin_id: SkinId,
    model: SkinModel,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct SkinIndex {
    skins: Vec<SkinAsset>,
    selections: Vec<AccountSkinSelection>,
}

impl SkinIndex {
    fn insert(&mut self, asset: SkinAsset) {
        if !self.skins.iter().any(|existing| existing.id == asset.id) {
            self.skins.push(asset);
        }
    }

    fn set_model(&mut self, skin_id: SkinId, model: SkinModel) {
        if let Some(asset) = self.skins.iter_mut().find(|asset| asset.id == skin_id) {
            asset.default_model = model;
        }
    }

    fn asset(&self, skin_id: SkinId) -> Option<SkinAsset> {
        self.skins.iter().find(|asset| asset.id == skin_id).cloned()
    }

    fn referenced_hashes(&self) -> HashSet<TextureHash> {
        self.skins.iter().map(|asset| asset.id.hash()).collect()
    }

    fn remove(&mut self, skin_id: SkinId) {
        self.skins.retain(|asset| asset.id != skin_id);
        self.selections
            .retain(|selection| selection.skin_id != skin_id);
    }

    fn selected_by(&self, account_id: AccountId) -> Option<&AccountSkinSelection> {
        self.selections
            .iter()
            .find(|selection| selection.account_id == account_id)
    }

    fn select(&mut self, selection: AccountSkinSelection) {
        self.clear_selection(selection.account_id);
        self.selections.push(selection);
    }

    fn clear_selection(&mut self, account_id: AccountId) {
        self.selections
            .retain(|selection| selection.account_id != account_id);
    }

    fn rename(&mut self, skin_id: SkinId, name: String) {
        if let Some(asset) = self.skins.iter_mut().find(|asset| asset.id == skin_id) {
            asset.name = name;
        }
    }
}

type SkinIndexStore =
    StoreLoader<NoMigrator, Store<SkinIndex>, SkinIndex, JsonFilePersistor<Store<SkinIndex>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedSkin {
    pub asset: SkinAsset,
    pub model: SkinModel,
    pub path: PathBuf,
}

/// A locally stored skin and the path of its PNG file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinEntry {
    pub asset: SkinAsset,
    pub path: PathBuf,
}

/// A local skin bound to an account-facing model choice.
pub struct LocalSkin<'a> {
    store: &'a SkinStore,
    skin_id: SkinId,
    model: SkinModel,
}

pub trait AccountSkinExt {
    /// Uploads `skin` to this account and remembers it as the account's selection.
    fn skin(&self, skin: LocalSkin<'_>) -> impl Future<Output = Result<()>> + Send;
}

impl<A> AccountSkinExt for A
where
    A: Account + ?Sized,
{
    async fn skin(&self, skin: LocalSkin<'_>) -> Result<()> {
        skin.store.upload(self, skin.skin_id, skin.model).await
    }
}

/// Stores skin PNG files locally by SHA-256 content hash and tracks
/// per-account selections.
pub struct SkinStore {
    scope: Scope,
    index: SkinIndexStore,
}

impl SkinStore {
    pub async fn open(scope: Scope) -> Result<Self> {
        let index = Store::load(
            NoMigrator,
            json_persistor("skins/index".to_owned(), scope.clone()),
            SKIN_INDEX_VERSION,
        )
        .await?;

        let store = Self { scope, index };
        store.gc().await?;
        Ok(store)
    }

    /// Builds an account-facing skin handle without touching the filesystem.
    pub fn skin(&self, skin_id: SkinId, model: SkinModel) -> LocalSkin<'_> {
        LocalSkin {
            store: self,
            skin_id,
            model,
        }
    }

    /// Imports PNG bytes into the local library.
    pub async fn import(
        &self,
        name: impl Into<String>,
        bytes: Vec<u8>,
        default_model: SkinModel,
    ) -> Result<SkinAsset> {
        let name = name.into();
        if !is_valid_name(&name) {
            return Err(Error::TextureNameEmpty);
        }
        if !bytes.starts_with(PNG_SIGNATURE) {
            return Err(Error::SkinNotPng { name });
        }

        let hash = TextureHash::of(&bytes);
        let id = SkinId::from_hash(hash);
        let path = self.path(id).await?;
        tokio::fs::write(&path, &bytes).await?;

        let inserted = SkinAsset {
            id,
            name,
            default_model,
        };

        self.index.set(move |index| index.insert(inserted)).await?;
        self.asset(id)
            .await
            .ok_or(Error::SkinNotFound { skin_id: id })
    }

    /// Renames a local skin without changing its id, PNG file, or selections.
    pub async fn rename(&self, skin_id: SkinId, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        if !is_valid_name(&name) {
            return Err(Error::TextureNameEmpty);
        }
        if self.asset(skin_id).await.is_none() {
            return Err(Error::SkinNotFound { skin_id });
        }

        self.index.set(|index| index.rename(skin_id, name)).await?;
        Ok(())
    }

    /// Updates the default model of a local skin without changing its content.
    pub async fn set_model(&self, skin_id: SkinId, model: SkinModel) -> Result<()> {
        if self.asset(skin_id).await.is_none() {
            return Err(Error::SkinNotFound { skin_id });
        }

        self.index
            .set(|index| index.set_model(skin_id, model))
            .await?;
        Ok(())
    }

    /// Lists local skins with their PNG paths without reading the files.
    pub async fn list(&self) -> Result<Vec<SkinEntry>> {
        let assets = self.index.get(|index| index.skins.clone()).await;
        let mut entries = Vec::with_capacity(assets.len());
        for asset in assets {
            let path = self.path(asset.id).await?;
            entries.push(SkinEntry { asset, path });
        }
        Ok(entries)
    }

    pub async fn remove(&self, skin_id: SkinId) -> Result<()> {
        let asset = self
            .asset(skin_id)
            .await
            .ok_or(Error::SkinNotFound { skin_id })?;

        let path = self.path(asset.id).await?;
        self.index.set(|index| index.remove(skin_id)).await?;
        remove_if_present(&path).await?;
        Ok(())
    }

    /// Removes unreferenced content files left behind by interrupted imports.
    async fn gc(&self) -> Result<()> {
        let referenced = self.index.get(SkinIndex::referenced_hashes).await;
        let root_path = self.scope.get_full_path("skins", None).await?;
        gc_content(&root_path, &referenced).await?;
        Ok(())
    }

    /// Selects a local skin for an account without uploading it.
    pub async fn select(
        &self,
        account_id: AccountId,
        skin_id: SkinId,
        model: SkinModel,
    ) -> Result<()> {
        if self.asset(skin_id).await.is_none() {
            return Err(Error::SkinNotFound { skin_id });
        }

        self.index
            .set(|index| {
                index.select(AccountSkinSelection {
                    account_id,
                    skin_id,
                    model,
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

    pub async fn selected(&self, account_id: AccountId) -> Result<Option<SelectedSkin>> {
        let Some(selection) = self.selection(account_id).await else {
            return Ok(None);
        };
        let (asset, path) = self.existing(selection.skin_id).await?;

        Ok(Some(SelectedSkin {
            asset,
            model: selection.model,
            path,
        }))
    }

    async fn asset(&self, skin_id: SkinId) -> Option<SkinAsset> {
        self.index.get(|index| index.asset(skin_id)).await
    }

    async fn selection(&self, account_id: AccountId) -> Option<AccountSkinSelection> {
        self.index
            .get(|index| index.selected_by(account_id).cloned())
            .await
    }

    async fn path(&self, skin_id: SkinId) -> Result<PathBuf> {
        let hash = skin_id.hash().hex();
        let prefix = &hash[..2];
        Ok(self
            .scope
            .get_full_path(&format!("skins/{prefix}/{hash}"), Some("png".to_owned()))
            .await?)
    }

    async fn existing(&self, skin_id: SkinId) -> Result<(SkinAsset, PathBuf)> {
        let asset = self
            .asset(skin_id)
            .await
            .ok_or(Error::SkinNotFound { skin_id })?;
        let path = self.path(skin_id).await?;
        if !tokio::fs::try_exists(&path).await? {
            return Err(Error::SkinFileNotFound { skin_id, path });
        }
        Ok((asset, path))
    }

    async fn upload<A>(&self, account: &A, skin_id: SkinId, model: SkinModel) -> Result<()>
    where
        A: Account + ?Sized,
    {
        let (_, path) = self.existing(skin_id).await?;
        let bytes = tokio::fs::read(&path).await?;
        account.upload_skin(bytes, model).await?;
        self.select(account.account_id(), skin_id, model).await
    }
}
