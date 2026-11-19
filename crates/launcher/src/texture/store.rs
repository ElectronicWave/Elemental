use std::path::PathBuf;

use elemental_core::auth::accounts::AccountId;
use elemental_shared::{
    migrator::NoMigrator,
    persistor::{JsonFilePersistor, json_persistor},
    scope::Scope,
    store::{Store, StoreLoader},
};
use tokio::sync::Mutex;

use super::{
    PNG_SIGNATURE, TextureAsset, TextureId, TextureIndex, TextureSelection, validate_name,
};
use crate::error::Result;

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
