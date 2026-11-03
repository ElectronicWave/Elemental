use scc::HashMap;
use scc::hash_map::Entry;
use std::{
    any::{Any, TypeId, type_name},
    future::Future,
    pin::Pin,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
};
#[cfg(feature = "notify")]
use tokio::sync::Notify;

use crate::error::{Error, Result};

pub static POOL: LazyLock<ObjectPool> = LazyLock::new(ObjectPool::new);
pub type ShutdownFn<T> =
    Box<dyn Fn(Arc<T>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
pub struct PoolEntry<T: Any + Send + Sync + ?Sized> {
    value: Option<Arc<T>>,
    #[cfg(feature = "notify")]
    notify: Arc<Notify>,
    active: AtomicBool,
    shutdown: Option<ShutdownFn<T>>,
}

struct CleanupTask<T: Any + Send + Sync + ?Sized> {
    value: Option<Arc<T>>,
    shutdown: Option<ShutdownFn<T>>,
    #[cfg(feature = "notify")]
    notify: Arc<Notify>,
}

impl<T: Any + Send + Sync + ?Sized> CleanupTask<T> {
    async fn run(self) {
        #[cfg(feature = "notify")]
        self.notify.notify_waiters();

        if let (Some(value), Some(shutdown)) = (self.value, self.shutdown) {
            shutdown(value).await;
        }
    }
}

impl<T: Any + Send + Sync + ?Sized> PoolEntry<T> {
    fn take_cleanup(&mut self) -> CleanupTask<T> {
        let value = self.value.take();
        let shutdown = self.shutdown.take();
        self.active.store(false, Ordering::Release);

        CleanupTask {
            value,
            shutdown,
            #[cfg(feature = "notify")]
            notify: self.notify.clone(),
        }
    }

    fn replace_with(&mut self, value: Arc<T>, shutdown: Option<ShutdownFn<T>>) -> CleanupTask<T> {
        let old_value = self.value.replace(value);
        let old_shutdown = std::mem::replace(&mut self.shutdown, shutdown);
        self.active.store(true, Ordering::Release);

        CleanupTask {
            value: old_value,
            shutdown: old_shutdown,
            #[cfg(feature = "notify")]
            notify: self.notify.clone(),
        }
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    fn new() -> Self {
        Self {
            value: None,
            #[cfg(feature = "notify")]
            notify: Arc::new(Notify::new()),
            active: AtomicBool::new(true),
            shutdown: None,
        }
    }
}

pub struct ObjectPool {
    inner: HashMap<TypeId, PoolEntry<dyn Any + Send + Sync>>,
}

impl Default for ObjectPool {
    fn default() -> Self {
        Self::new()
    }
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
        let cleanup = match self.inner.entry_async(type_id).await {
            Entry::Occupied(mut entry) => Some(entry.get_mut().replace_with(value, shutdown)),
            Entry::Vacant(entry) => {
                let mut pool_entry = PoolEntry::new();
                pool_entry.value = Some(value as Arc<dyn Any + Send + Sync>);
                pool_entry.shutdown = shutdown;
                entry.insert_entry(pool_entry);
                None
            }
        };

        if let Some(cleanup) = cleanup {
            cleanup.run().await;
        }
    }

    pub async fn remove_value<T: Any + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let cleanup = if let Some(mut entry) = self.inner.get_async(&type_id).await {
            let pool_entry = entry.get_mut();
            let had_value = pool_entry.value.is_some();
            Some((had_value, pool_entry.take_cleanup()))
        } else {
            None
        };

        if let Some((had_value, cleanup)) = cleanup {
            cleanup.run().await;
            return had_value;
        }

        false
    }

    pub async fn remove_entry<T: Any + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        if let Some((_, mut entry)) = self.inner.remove_async(&type_id).await {
            entry.take_cleanup().run().await;
            return true;
        }

        false
    }

    #[cfg(feature = "notify")]
    pub async fn wait_value<T: Any + Send + Sync>(&self) {
        let type_id = TypeId::of::<T>();

        loop {
            // Register and read the state under the same entry lock. This keeps the
            // notification handle tied to the entry whose state is observed, even if
            // the entry is removed and recreated concurrently.
            let entry = self
                .inner
                .entry_async(type_id)
                .await
                .or_insert_with(PoolEntry::new);
            let (notified, state) = {
                let pool_entry = entry.get();
                (
                    pool_entry.notify.clone().notified_owned(),
                    (pool_entry.is_active(), pool_entry.value.is_some()),
                )
            };
            drop(entry);

            match state {
                (false, _) | (true, true) => return,
                (true, false) => notified.await,
            }
        }
    }

    pub async fn fulfill_value<T: Any + Send + Sync>(
        &self,
        value: Arc<T>,
        shutdown: Option<ShutdownFn<T>>,
    ) -> Result<bool> {
        let type_id = TypeId::of::<T>();
        let shutdown = shutdown.map(|shutdown_fn| {
            Box::new(move |value_any: Arc<dyn Any + Send + Sync>| {
                let value = value_any
                    .downcast::<T>()
                    .expect("Can't downcast value in shutdown fn");
                shutdown_fn(value)
            }) as ShutdownFn<dyn Any + Send + Sync>
        });
        let Some(mut entry) = self.inner.get_async(&type_id).await else {
            return Err(Error::NoWaitingEntry {
                type_name: type_name::<T>(),
            });
        };

        let cleanup = {
            let pool_entry = entry.get_mut();
            if pool_entry.value.is_some() {
                return Ok(false);
            }
            if !pool_entry.is_active() {
                return Err(Error::EntryClosed {
                    type_name: type_name::<T>(),
                });
            }
            pool_entry.replace_with(value, shutdown)
        };

        drop(entry);
        cleanup.run().await;
        Ok(true)
    }

    pub async fn shutdown(&self) {
        let mut cleanups = Vec::new();
        let mut iter = self.inner.begin_async().await;
        while let Some(mut entry) = iter {
            cleanups.push(entry.get_mut().take_cleanup());
            iter = entry.next_async().await;
        }

        for cleanup in cleanups {
            cleanup.run().await;
        }
    }
}
