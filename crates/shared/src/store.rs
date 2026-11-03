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

pub type StoreLoader<M, S, V, P> = Loader<M, S, Store<V>, P>;

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
