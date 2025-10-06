use anyhow::{Context, Result};
use scc::HashMap;
use std::{
    any::{type_name, Any, TypeId},
    sync::{Arc, LazyLock},
};
const POOL: LazyLock<ObjectPool> = LazyLock::new(ObjectPool::new);

type Value = Arc<dyn Any + Send + Sync>;

/// Object Pool Library
struct ObjectPool {
    inner: HashMap<TypeId, Value>,
}

impl ObjectPool {
    /// Create a new Object Pool
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn get_sync<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.inner.get_sync(&TypeId::of::<T>()).and_then(|entry| {
            let value: Arc<dyn Any + Send + Sync> = entry.get().clone();
            Arc::downcast::<T>(value).ok()
        })
    }

    pub async fn get_async<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.inner
            .get_async(&TypeId::of::<T>())
            .await
            .and_then(|entry| {
                let value: Arc<dyn Any + Send + Sync> = entry.get().clone();
                Arc::downcast::<T>(value).ok()
            })
    }

    pub async fn build<T: Any + Send + Sync, F>(&self, builder: F) -> Arc<T>
    where
        F: FnOnce() -> T,
    {
        let type_id = TypeId::of::<T>();
        let value = Arc::new(builder());
        self.inner.upsert_async(type_id, value.clone()).await;
        value
    }
}

pub async fn require<T: Any + Send + Sync>() -> Result<Arc<T>> {
    POOL.get_async::<T>().await.context(format!(
        "Cannot get object `{}` from pool",
        type_name::<T>()
    ))
}

#[cfg(test)]
mod testobj {

    use super::*;
    #[tokio::test]
    async fn test() {
        let a = require::<String>().await;
        println!("{:?}", a);
    }
}
