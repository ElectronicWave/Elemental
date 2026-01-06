use std::sync::Arc;

use crate::{
    profile::Profile,
    version::{Migrator, Persistor, VersionControlled},
};
use anyhow::Result;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Loader<M: Migrator<V>, V: VersionControlled, P: Persistor<V>> {
    pub inner: Arc<RwLock<V>>,
    pub migrator: M,
    pub persistor: P,
}

impl<M: Migrator<V>, V: VersionControlled, P: Persistor<V>> Loader<M, V, P> {
    pub async fn load(migrator: M, persistor: P, loader_version: usize) -> Result<Self> {
        let mut value = persistor.load().await?.unwrap_or_default();

        if !value.is_up_to_date(loader_version) {
            value = migrator.migrate(value, loader_version)?;
            // Save migrated value
            persistor.save(&value).await?;
        }

        Ok(Self {
            inner: Arc::new(RwLock::new(value)),
            migrator,
            persistor,
        })
    }

    pub async fn cloned(&self) -> V
    where
        V: Clone,
    {
        let guard = self.inner.read().await;
        guard.clone()
    }

    pub async fn get<T>(&self, f: impl FnOnce(&V) -> T) -> T {
        let guard = self.inner.read().await;
        f(&*guard)
    }

    pub async fn set(&self, f: impl FnOnce(&mut V)) -> Result<()> {
        let mut guard = self.inner.write().await;
        f(&mut *guard);
        self.persistor.save(&*guard).await?;
        Ok(())
    }
}

pub type ProfileLoader<M, C, P> = Loader<M, Profile<C>, P>;
