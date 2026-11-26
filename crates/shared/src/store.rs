use crate::{
    error::Result,
    loader::Loader,
    migrator::Migrator,
    version::{Persistor, VersionControlled, Versioned},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Store<V> {
    pub value: V,
    pub version: usize,
}

impl<T> Versioned for Store<T> {
    #[inline(always)]
    fn version(&self) -> usize {
        self.version
    }
}

impl<T> VersionControlled for Store<T> {
    #[inline(always)]
    fn with_version(mut self, version: usize) -> Self {
        self.version = version;
        self
    }
}

pub struct StoreLoader<M, S, V, P>
where
    M: Migrator<S, Store<V>>,
    S: Versioned,
    P: Persistor<S>,
{
    loader: Loader<M, S, Store<V>, P>,
}

impl<M, S, V, P> Clone for StoreLoader<M, S, V, P>
where
    M: Migrator<S, Store<V>>,
    S: Versioned,
    P: Persistor<S>,
{
    fn clone(&self) -> Self {
        Self {
            loader: self.loader.clone(),
        }
    }
}

impl<M, S, V, P> StoreLoader<M, S, V, P>
where
    M: Migrator<S, Store<V>>,
    S: Versioned,
    V: Default,
    P: Persistor<S>,
{
    pub async fn load(migrator: M, persistor: P, target_version: usize) -> Result<Self> {
        Ok(Self {
            loader: Loader::load(migrator, persistor, target_version).await?,
        })
    }

    pub async fn cloned(&self) -> V
    where
        V: Clone,
    {
        self.loader.get(|store| store.value.clone()).await
    }

    pub async fn version(&self) -> usize {
        self.loader.get(|store| store.version).await
    }

    pub async fn get<T>(&self, f: impl FnOnce(&V) -> T) -> T {
        self.loader.get(|store| f(&store.value)).await
    }

    pub async fn set(&self, f: impl FnOnce(&mut V)) -> Result<()> {
        self.loader.set(|store| f(&mut store.value)).await
    }
}

impl<V: Default> Store<V> {
    pub async fn load<M, S, P>(
        migrator: M,
        persistor: P,
        target_version: usize,
    ) -> Result<StoreLoader<M, S, V, P>>
    where
        S: Versioned,
        M: Migrator<S, Store<V>>,
        P: Persistor<S>,
    {
        StoreLoader::load(migrator, persistor, target_version).await
    }
}

impl<V: Clone> Clone for Store<V> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            version: self.version,
        }
    }
}
