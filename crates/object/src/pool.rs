use scc::HashMap;
use std::{
    any::{Any, TypeId},
    future::Future,
    pin::Pin,
    sync::{Arc, LazyLock},
};
#[cfg(feature = "notify")]
use tokio::sync::Notify;

pub static POOL: LazyLock<ObjectPool> = LazyLock::new(|| ObjectPool::new());
pub type ShutdownFn<T> =
    Box<dyn Fn(Arc<T>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
pub struct PoolEntry<T: Any + Send + Sync + ?Sized> {
    value: Option<Arc<T>>,
    #[cfg(feature = "notify")]
    notify: Arc<Notify>,
    shutdown: Option<ShutdownFn<T>>,
}

impl<T: Any + Send + Sync + ?Sized> PoolEntry<T> {
    async fn shutdown(&mut self) {
        let val = self.value.clone();
        self.value = None;
        if let Some(shutdown) = &self.shutdown {
            if let Some(value) = &val {
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

pub struct ObjectPool {
    inner: HashMap<TypeId, PoolEntry<dyn Any + Send + Sync>>,
}

impl ObjectPool {
    /// Create a new Object Pool
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub async fn get_async<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        let typeid = TypeId::of::<T>();
        let entry = self.inner.read_async(&typeid, |_, v| v.value.clone()).await;
        if let Some(entry) = entry {
            return Arc::downcast::<T>(entry?).ok();
        }

        None
    }

    // Overwrite and call the notifier if exists, otherwise just insert the value.
    pub async fn set_value<T: Any + Send + Sync>(
        &self,
        value: Arc<T>,
        shutdown: Option<ShutdownFn<T>>,
    ) {
        let type_id = TypeId::of::<T>();
        let shutdown = shutdown.map(|shutdown_fn| {
            Box::new(move |value_any: Arc<dyn Any + Send + Sync>| {
                let value = value_any
                    .downcast::<T>()
                    .expect("Can't downcast value in shutdown fn");
                shutdown_fn(value)
            }) as ShutdownFn<dyn Any + Send + Sync>
        });
        if self.inner.contains_async(&type_id).await {
            // Call shutdown of old value if exists, and update the value and shutdown fn.
            // It means the old value will be shutdown before the new value is set, so the shutdown fn can still access the old value.
            let mut entry = self.inner.get_async(&type_id).await.unwrap();
            entry.shutdown().await;
            entry.value = Some(value.clone() as Arc<dyn Any + Send + Sync>);
            entry.shutdown = shutdown;
            #[cfg(feature = "notify")]
            entry.notify.notify_waiters();
            return;
        }

        let mut entry = PoolEntry::new();
        entry.value = Some(value.clone() as Arc<dyn Any + Send + Sync>);
        entry.shutdown = shutdown;
        // The entry is new, just insert it!
        self.inner.upsert_async(type_id, entry).await;
    }

    pub async fn remove_value<T: Any + Send + Sync>(&self) {
        let type_id = TypeId::of::<T>();
        if let Some(mut entry) = self.inner.get_async(&type_id).await {
            entry.shutdown().await;
        }
    }

    pub async fn remove_entry<T: Any + Send + Sync>(&self) {
        let type_id = TypeId::of::<T>();
        if let Some((_, mut entry)) = self.inner.remove_async(&type_id).await {
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
    pub async fn fulfill_value<T: Any + Send + Sync>(
        &self,
        value: Arc<T>,
        shutdown: Option<ShutdownFn<T>>,
    ) {
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
            entry.value = Some(value.clone() as Arc<dyn Any + Send + Sync>);
            let shutdown = shutdown.map(|shutdown_fn| {
                Box::new(move |value_any: Arc<dyn Any + Send + Sync>| {
                    let value = value_any
                        .downcast::<T>()
                        .expect("Can't downcast value in shutdown fn");
                    shutdown_fn(value)
                }) as ShutdownFn<dyn Any + Send + Sync>
            });
            entry.shutdown = shutdown;
            entry.notify.notify_waiters();
        }
    }

    pub async fn shutdown(&self) {
        let mut iter = self.inner.begin_async().await;
        while let Some(mut entry) = iter {
            // `OccupiedEntry` can be sent across awaits and threads.
            entry.shutdown().await;
            iter = entry.next_async().await;
        }
    }
}
