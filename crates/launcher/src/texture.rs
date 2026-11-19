use std::{fmt, path::PathBuf};

use elemental_core::auth::accounts::AccountId;
use elemental_shared::{
    migrator::NoMigrator,
    persistor::{JsonFilePersistor, json_persistor},
    scope::Scope,
    store::{Store, StoreLoader},
};
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;

use crate::error::{Error, Result};

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(Error::TextureNameEmpty);
    }
    Ok(())
}

pub(crate) trait TextureId: Copy + Eq + fmt::Display + Send + Sync + 'static {
    fn from_u64(value: u64) -> Self;
}

pub(crate) trait TextureAsset:
    Clone + Serialize + DeserializeOwned + Send + Sync + 'static
{
    type Id: TextureId;

    fn id(&self) -> Self::Id;

    fn rename(&mut self, name: String);
}

pub(crate) trait TextureSelection:
    Clone + Serialize + DeserializeOwned + Send + Sync + 'static
{
    type Id: TextureId;

    fn account_id(&self) -> AccountId;

    fn texture_id(&self) -> Self::Id;
}

pub(crate) trait TextureIndex:
    Clone + Default + Serialize + DeserializeOwned + Send + Sync + 'static
{
    type Id: TextureId;
    type Asset: TextureAsset<Id = Self::Id>;
    type Selection: TextureSelection<Id = Self::Id>;

    fn parts(&self) -> (&u64, &Vec<Self::Asset>, &Vec<Self::Selection>);

    fn parts_mut(&mut self) -> (&mut u64, &mut Vec<Self::Asset>, &mut Vec<Self::Selection>);

    fn not_found(id: Self::Id) -> Error;

    fn file_not_found(id: Self::Id, path: PathBuf) -> Error;

    fn not_png(name: String) -> Error;

    fn id_exhausted() -> Error;

    fn next_id(&self) -> u64 {
        *self.parts().0
    }

    fn set_next_id(&mut self, next_id: u64) {
        *self.parts_mut().0 = next_id;
    }

    fn assets(&self) -> &[Self::Asset] {
        self.parts().1
    }

    fn assets_mut(&mut self) -> &mut Vec<Self::Asset> {
        self.parts_mut().1
    }

    fn remove(&mut self, id: Self::Id) {
        let (_, assets, selections) = self.parts_mut();
        assets.retain(|asset| asset.id() != id);
        selections.retain(|selection| selection.texture_id() != id);
    }

    fn selected_by(&self, account_id: AccountId) -> Option<&Self::Selection> {
        self.parts()
            .2
            .iter()
            .find(|selection| selection.account_id() == account_id)
    }

    fn select(&mut self, selection: Self::Selection) {
        let account_id = selection.account_id();
        self.clear_selection(account_id);
        self.parts_mut().2.push(selection);
    }

    fn clear_selection(&mut self, account_id: AccountId) {
        self.parts_mut()
            .2
            .retain(|selection| selection.account_id() != account_id);
    }

    fn rename(&mut self, id: Self::Id, name: String) {
        if let Some(asset) = self.parts_mut().1.iter_mut().find(|asset| asset.id() == id) {
            asset.rename(name);
        }
    }
}

type TextureIndexStore<I> = StoreLoader<NoMigrator, Store<I>, I, JsonFilePersistor<Store<I>>>;

pub(crate) struct TextureStore<I>
where
    I: TextureIndex,
{
    scope: Scope,
    directory: &'static str,
    index: TextureIndexStore<I>,
    import_lock: Mutex<()>,
}

impl<I> TextureStore<I>
where
    I: TextureIndex,
{
    pub(crate) async fn open(
        scope: Scope,
        directory: &'static str,
        version: usize,
    ) -> Result<Self> {
        let index = Store::load(
            NoMigrator,
            json_persistor(format!("{directory}/index"), scope.clone()),
            version,
        )
        .await?;

        Ok(Self {
            scope,
            directory,
            index,
            import_lock: Mutex::new(()),
        })
    }

    pub(crate) async fn import<F>(
        &self,
        name: impl Into<String>,
        bytes: Vec<u8>,
        build: F,
    ) -> Result<I::Asset>
    where
        F: FnOnce(I::Id, String) -> I::Asset,
    {
        let name = name.into();
        validate_name(&name)?;
        if !bytes.starts_with(PNG_SIGNATURE) {
            return Err(I::not_png(name));
        }

        let _guard = self.import_lock.lock().await;
        let next_id = self
            .index
            .get(|index| index.value.next_id().checked_add(1))
            .await
            .ok_or_else(I::id_exhausted)?;
        let asset = build(I::Id::from_u64(next_id), name);

        let path = self.path(asset.id()).await?;
        tokio::fs::write(&path, bytes).await?;

        self.index
            .set(|index| {
                index.value.set_next_id(next_id);
                index.value.assets_mut().push(asset.clone());
            })
            .await?;
        Ok(asset)
    }

    pub(crate) async fn rename(&self, id: I::Id, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        validate_name(&name)?;
        if self.asset(id).await.is_none() {
            return Err(I::not_found(id));
        }

        self.index.set(|index| index.value.rename(id, name)).await?;
        Ok(())
    }

    pub(crate) async fn list(&self) -> Result<Vec<(I::Asset, PathBuf)>> {
        let assets = self.index.get(|index| index.value.assets().to_vec()).await;
        let mut entries = Vec::with_capacity(assets.len());
        for asset in assets {
            let path = self.path(asset.id()).await?;
            entries.push((asset, path));
        }
        Ok(entries)
    }

    pub(crate) async fn remove(&self, id: I::Id) -> Result<()> {
        let asset = self.asset(id).await.ok_or_else(|| I::not_found(id))?;

        let path = self.path(asset.id()).await?;
        self.index.set(|index| index.value.remove(id)).await?;
        tokio::fs::remove_file(path).await?;
        Ok(())
    }

    pub(crate) async fn select(&self, selection: I::Selection) -> Result<()> {
        let id = selection.texture_id();
        if self.asset(id).await.is_none() {
            return Err(I::not_found(id));
        }

        self.index
            .set(|index| index.value.select(selection))
            .await?;
        Ok(())
    }

    pub(crate) async fn clear_selection(&self, account_id: AccountId) -> Result<()> {
        self.index
            .set(|index| index.value.clear_selection(account_id))
            .await?;
        Ok(())
    }

    pub(crate) async fn selected(
        &self,
        account_id: AccountId,
    ) -> Result<Option<(I::Selection, I::Asset, PathBuf)>> {
        let Some(selection) = self.selection(account_id).await else {
            return Ok(None);
        };
        let (asset, path) = self.existing(selection.texture_id()).await?;

        Ok(Some((selection, asset, path)))
    }

    pub(crate) async fn existing(&self, id: I::Id) -> Result<(I::Asset, PathBuf)> {
        let asset = self.asset(id).await.ok_or_else(|| I::not_found(id))?;
        let path = self.path(id).await?;
        if !tokio::fs::try_exists(&path).await? {
            return Err(I::file_not_found(id, path));
        }
        Ok((asset, path))
    }

    async fn asset(&self, id: I::Id) -> Option<I::Asset> {
        self.index
            .get(|index| {
                index
                    .value
                    .assets()
                    .iter()
                    .find(|asset| asset.id() == id)
                    .cloned()
            })
            .await
    }

    async fn selection(&self, account_id: AccountId) -> Option<I::Selection> {
        self.index
            .get(|index| index.value.selected_by(account_id).cloned())
            .await
    }

    async fn path(&self, id: I::Id) -> Result<PathBuf> {
        Ok(self
            .scope
            .get_full_path(
                &format!("{}/files/{id}", self.directory),
                Some("png".to_string()),
            )
            .await?)
    }
}
