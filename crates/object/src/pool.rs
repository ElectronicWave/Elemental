use anyhow::{Context, Result};
use scc::HashMap;
use std::{
    any::{Any, TypeId, type_name},
    future::Future,
    pin::Pin,
    sync::{Arc, LazyLock},
};
#[cfg(feature = "notify")]
use tokio::sync::Notify;
static POOL: LazyLock<ObjectPool> = LazyLock::new(|| ObjectPool::new());
type ShutdownFn<T> = Box<dyn Fn(Arc<T>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
struct PoolEntry<T: Any + Send + Sync + ?Sized> {
    value: Option<Arc<T>>,
    #[cfg(feature = "notify")]
    notify: Arc<Notify>,
    shutdown: Option<ShutdownFn<T>>,
}

impl<T: Any + Send + Sync + ?Sized> PoolEntry<T> {
    async fn shutdown(&self) {
        if let Some(shutdown) = &self.shutdown {
            if let Some(value) = &self.value {
                (shutdown)(value.clone()).await;
            }
        }
    }

    fn new() -> Self {
        Self {
            value: None,
            #[cfg(feature = "notify")]
            notify: Arc::new(Notify::new()),
            shutdown: None,
        }
    }
}

struct ObjectPool {
    inner: HashMap<TypeId, PoolEntry<dyn Any + Send + Sync>>,
}

impl ObjectPool {
    /// Create a new Object Pool
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn get_sync<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.inner
            .get_sync(&TypeId::of::<T>())
            .and_then(|entry| Arc::downcast::<T>(entry.get().value.clone()?).ok())
    }

    pub async fn get_async<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        let typeid = TypeId::of::<T>();
        let entry = self.inner.read_async(&typeid, |_, v| v.value.clone()).await;
        if let Some(entry) = entry {
            return Arc::downcast::<T>(entry?).ok();
        }

        None
    }

    pub async fn set_value<T: Any + Send + Sync>(
        &self,
        value: Arc<T>,
        shutdown: Option<ShutdownFn<T>>,
    ) {
        let type_id = TypeId::of::<T>();
        let mut entry = PoolEntry::new();
        entry.value = Some(value.clone() as Arc<dyn Any + Send + Sync>);
        entry.shutdown = shutdown.map(|shutdown_fn| {
            Box::new(move |value_any: Arc<dyn Any + Send + Sync>| {
                let value = value_any
                    .downcast::<T>()
                    .expect("Can't downcast value in shutdown fn");
                shutdown_fn(value)
            }) as ShutdownFn<dyn Any + Send + Sync>
        });

        let popout = self.inner.upsert_async(type_id, entry).await;
        if let Some(old_entry) = popout {
            old_entry.shutdown().await;
        }
    }

    pub async fn remove_value<T: Any + Send + Sync>(&self) {
        let type_id = TypeId::of::<T>();
        if let Some(mut entry) = self.inner.get_async(&type_id).await {
            entry.shutdown().await;
            entry.value = None;
        }
    }

    pub async fn remove_entry<T: Any + Send + Sync>(&self) {
        let type_id = TypeId::of::<T>();
        if let Some((_, entry)) = self.inner.remove_async(&type_id).await {
            entry.shutdown().await;
        }
    }

    #[cfg(feature = "notify")]
    pub async fn wait_value<T: Any + Send + Sync>(&self) {
        let type_id = TypeId::of::<T>();
        let notified = self
            .inner
            .entry_async(type_id)
            .await
            .or_insert(PoolEntry::new())
            .get()
            .notify
            .clone();
        notified.notified().await;
    }

    #[cfg(feature = "notify")]
    pub async fn fulfill_value<T: Any + Send + Sync>(&self, value: Arc<T>) {
        let type_id = TypeId::of::<T>();
        if let Some(fulfilled) = self
            .inner
            .read_async(&type_id, |_, v| v.value.is_some())
            .await
            && fulfilled
        {
            // Noops if already fulfilled
            return;
        }

        if let Some(mut entry) = self.inner.get_async(&type_id).await {
            entry.value = Some(value);
            entry.notify.notify_waiters();
        }
    }
}

pub async fn require<T: Any + Send + Sync>() -> Result<Arc<T>> {
    POOL.get_async::<T>().await.context(format!(
        "Cannot get object `{}` from pool",
        type_name::<T>()
    ))
}
/// Provide a value to the pool, it will let the old value shutdown if exists.
pub async fn provide<T, F, Fut>(value: T, shutdown: Option<F>)
where
    T: Any + Send + Sync,
    F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    provide_arc(Arc::new(value), shutdown).await;
}

/// Provide a value to the pool, it will let the old value shutdown if exists.
pub async fn provide_arc<T, F, Fut>(value: Arc<T>, shutdown: Option<F>)
where
    T: Any + Send + Sync,
    F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let shutdown: Option<ShutdownFn<T>> = shutdown.map(|f| {
        Box::new(move |value: Arc<T>| {
            Box::pin(f(value)) as Pin<Box<dyn Future<Output = ()> + Send>>
        }) as ShutdownFn<T>
    });
    POOL.set_value(value, shutdown).await;
}

pub fn require_sync<T: Any + Send + Sync>() -> Result<Arc<T>> {
    POOL.get_sync::<T>().context(format!(
        "Cannot get object `{}` from pool",
        type_name::<T>()
    ))
}

pub async fn drop_value<T: Any + Send + Sync>() {
    POOL.remove_value::<T>().await;
}

pub async fn drop_entry<T: Any + Send + Sync>() {
    POOL.remove_entry::<T>().await;
}
/// Acquire a value from the pool, if not exists, wait until it is provided.
#[cfg(feature = "notify")]
pub async fn acquire<T: Any + Send + Sync>() -> Result<Arc<T>> {
    POOL.wait_value::<T>().await;
    require::<T>().await
}

#[cfg(feature = "notify")]
pub async fn fulfill<T: Any + Send + Sync>(value: T) {
    fulfill_arc(Arc::new(value)).await;
}

#[cfg(feature = "notify")]
pub async fn fulfill_arc<T: Any + Send + Sync>(value: Arc<T>) {
    POOL.fulfill_value(value).await;
}
