use std::{fmt, future::Future, path::PathBuf};

use elemental_core::auth::{
    accounts::{Account, AccountId},
    texture::SkinModel,
};
use elemental_shared::scope::Scope;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    texture::{TextureAsset, TextureId, TextureIndex, TextureSelection, TextureStore},
};

const SKIN_INDEX_VERSION: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SkinId(u64);

impl fmt::Display for SkinId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl TextureId for SkinId {
    fn from_u64(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkinAsset {
    pub id: SkinId,
    pub name: String,
    pub default_model: SkinModel,
}

impl TextureAsset for SkinAsset {
    type Id = SkinId;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn rename(&mut self, name: String) {
        self.name = name;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AccountSkinSelection {
    account_id: AccountId,
    skin_id: SkinId,
    model: SkinModel,
}

impl TextureSelection for AccountSkinSelection {
    type Id = SkinId;

    fn account_id(&self) -> AccountId {
        self.account_id
    }

    fn texture_id(&self) -> Self::Id {
        self.skin_id
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct SkinIndex {
    next_id: u64,
    skins: Vec<SkinAsset>,
    selections: Vec<AccountSkinSelection>,
}

impl TextureIndex for SkinIndex {
    type Id = SkinId;
    type Asset = SkinAsset;
    type Selection = AccountSkinSelection;

    fn parts(&self) -> (&u64, &Vec<Self::Asset>, &Vec<Self::Selection>) {
        (&self.next_id, &self.skins, &self.selections)
    }

    fn parts_mut(&mut self) -> (&mut u64, &mut Vec<Self::Asset>, &mut Vec<Self::Selection>) {
        (&mut self.next_id, &mut self.skins, &mut self.selections)
    }

    fn not_found(skin_id: SkinId) -> Error {
        Error::SkinNotFound { skin_id }
    }

    fn file_not_found(skin_id: SkinId, path: PathBuf) -> Error {
        Error::SkinFileNotFound { skin_id, path }
    }

    fn not_png(name: String) -> Error {
        Error::SkinNotPng { name }
    }

    fn id_exhausted() -> Error {
        Error::SkinIdExhausted
    }
}

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
    inner: TextureStore<SkinIndex>,
}

impl SkinStore {
    pub async fn open(scope: Scope) -> Result<Self> {
        Ok(Self {
            inner: TextureStore::open(scope, "skins", SKIN_INDEX_VERSION).await?,
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
        self.inner
            .import(name, bytes, |id, name| SkinAsset {
                id,
                name,
                default_model,
            })
            .await
    }

    /// Renames a local skin without changing its id, PNG file, or selections.
    pub async fn rename(&self, skin_id: SkinId, name: impl Into<String>) -> Result<()> {
        self.inner.rename(skin_id, name).await
    }

    /// Lists local skins with their PNG paths without reading the files.
    pub async fn list(&self) -> Result<Vec<SkinEntry>> {
        Ok(self
            .inner
            .list()
            .await?
            .into_iter()
            .map(|(asset, path)| SkinEntry { asset, path })
            .collect())
    }

    pub async fn remove(&self, skin_id: SkinId) -> Result<()> {
        self.inner.remove(skin_id).await
    }

    /// Selects a local skin for an account without uploading it.
    pub async fn select(
        &self,
        account_id: AccountId,
        skin_id: SkinId,
        model: SkinModel,
    ) -> Result<()> {
        self.inner
            .select(AccountSkinSelection {
                account_id,
                skin_id,
                model,
            })
            .await
    }

    pub async fn clear_selection(&self, account_id: AccountId) -> Result<()> {
        self.inner.clear_selection(account_id).await
    }

    pub async fn selected(&self, account_id: AccountId) -> Result<Option<SelectedSkin>> {
        Ok(self
            .inner
            .selected(account_id)
            .await?
            .map(|(selection, asset, path)| SelectedSkin {
                asset,
                model: selection.model,
                path,
            }))
    }

    async fn upload<A>(&self, account: &A, skin_id: SkinId, model: SkinModel) -> Result<()>
    where
        A: Account + ?Sized,
    {
        let (_, path) = self.inner.existing(skin_id).await?;
        let bytes = tokio::fs::read(&path).await?;
        account.upload_skin(bytes, model).await?;
        self.select(account.account_id(), skin_id, model).await
    }
}
