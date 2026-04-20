use crate::{
    loader::Loader,
    version::{Migrator, Persistor, VersionControlled},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Store<V> {
    pub value: V,
    pub version: usize,
}

impl<T: Default> VersionControlled for Store<T> {
    #[inline(always)]
    fn version(&self) -> usize {
        self.version
    }
}

pub type StoreLoader<M, V, P> = Loader<M, Store<V>, P>;

impl<V: Default> Store<V> {
    pub async fn load<M: Migrator<Store<V>>, P: Persistor<Store<V>>>(
        migrator: M,
        persistor: P,
        version: usize,
    ) -> Result<StoreLoader<M, V, P>> {
        StoreLoader::load(migrator, persistor, version).await
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
