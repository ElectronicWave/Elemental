use std::{fmt, future::Future, path::PathBuf};

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
use tokio::sync::Mutex;

use crate::error::{Error, Result};

const SKIN_INDEX_VERSION: usize = 1;
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SkinId(u64);

impl fmt::Display for SkinId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkinAsset {
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
    next_id: u64,
    skins: Vec<SkinAsset>,
    selections: Vec<AccountSkinSelection>,
}

impl SkinIndex {
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

/// Stores skin PNG files locally and tracks per-account selections.
pub struct SkinStore {
    scope: Scope,
    index: SkinIndexStore,
    import_lock: Mutex<()>,
}

impl SkinStore {
    pub async fn open(scope: Scope) -> Result<Self> {
        let index = Store::load(
            NoMigrator,
            json_persistor("skins/index".to_owned(), scope.clone()),
            SKIN_INDEX_VERSION,
        )
        .await?;

        Ok(Self {
            scope,
            index,
            import_lock: Mutex::new(()),
        })
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
        if !bytes.starts_with(PNG_SIGNATURE) {
            return Err(Error::SkinNotPng { name });
        }

        let _guard = self.import_lock.lock().await;
        let next_id = self
            .index
            .get(|index| index.value.next_id.checked_add(1))
            .await
            .ok_or(Error::SkinIdExhausted)?;
        let asset = SkinAsset {
            id: SkinId(next_id),
            name,
            default_model,
        };

        let path = self.path(asset.id).await?;
        tokio::fs::write(&path, bytes).await?;

        self.index
            .set(|index| {
                index.value.next_id = next_id;
                index.value.skins.push(asset.clone());
            })
            .await?;
        Ok(asset)
    }

    /// Lists local skins with their PNG paths without reading the files.
    pub async fn list(&self) -> Result<Vec<SkinEntry>> {
        let assets = self.index.get(|index| index.value.skins.clone()).await;
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
        self.index.set(|index| index.value.remove(skin_id)).await?;
        tokio::fs::remove_file(path).await?;
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
                index.value.select(AccountSkinSelection {
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
            .set(|index| index.value.clear_selection(account_id))
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
        self.index
            .get(|index| {
                index
                    .value
                    .skins
                    .iter()
                    .find(|asset| asset.id == skin_id)
                    .cloned()
            })
            .await
    }

    async fn selection(&self, account_id: AccountId) -> Option<AccountSkinSelection> {
        self.index
            .get(|index| index.value.selected_by(account_id).cloned())
            .await
    }

    async fn path(&self, skin_id: SkinId) -> Result<PathBuf> {
        Ok(self
            .scope
            .get_full_path(&format!("skins/files/{skin_id}"), Some("png".to_owned()))
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
