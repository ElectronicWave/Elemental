use std::{
    any::{Any, type_name},
    future::Future,
    pin::Pin,
    sync::Arc,
};

pub use crate::context::ObjectContext;
use crate::error::{Error, Result};
use crate::pool::{ObjectPool, POOL, ShutdownFn};
use crate::{context::CONTEXT, instant::InstantObject};

/// A callback invoked when a pooled value is replaced or removed.
///
/// Implemented automatically for asynchronous closures of the shape
/// `Fn(Arc<T>) -> Future<Output = ()>`.
pub trait ShutdownCallback<T>: Send + Sync + 'static
where
    T: Any + Send + Sync,
{
    /// Future produced by the callback.
    type Future: Future<Output = ()> + Send + 'static;

    /// Invoke the callback with the value being shut down.
    fn call(&self, value: Arc<T>) -> Self::Future;
}

impl<T, F, Fut> ShutdownCallback<T> for F
where
    T: Any + Send + Sync,
    F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    type Future = Fut;

    fn call(&self, value: Arc<T>) -> Self::Future {
        self(value)
    }
}

async fn get_from_pool<T: Any + Send + Sync>(pool: &ObjectPool) -> Result<Arc<T>> {
    pool.get_async::<T>().await.ok_or(Error::ObjectNotFound {
        type_name: type_name::<T>(),
    })
}

/// Fast try to get a value from the pool, if not exists, return error immediately.
pub async fn require<T: Any + Send + Sync>() -> Result<Arc<T>> {
    if let Some(pool) = context_pool()
        && let Ok(value) = get_from_pool::<T>(&pool).await
    {
        return Ok(value);
    }

    get_from_pool::<T>(&POOL).await
}

/// Provide a value to the pool, it will let the old value shutdown if exists.
/// Provide value to a existing value means safe **HOT RELOAD**
pub async fn provide<T>(value: T)
where
    T: Any + Send + Sync,
{
    provide_arc(&POOL, Arc::new(value), None).await;
}

pub async fn provide_with_shutdown<T, F>(value: T, shutdown: F)
where
    T: Any + Send + Sync,
    F: ShutdownCallback<T>,
{
    provide_arc_with(&POOL, value, shutdown).await;
}

pub async fn provide_context<T>(value: T)
where
    T: Any + Send + Sync,
{
    let context = context_pool();
    provide_arc(context.as_deref().unwrap_or(&POOL), Arc::new(value), None).await;
}

pub async fn provide_context_with_shutdown<T, F>(value: T, shutdown: F)
where
    T: Any + Send + Sync,
    F: ShutdownCallback<T>,
{
    let context = context_pool();
    provide_arc_with(context.as_deref().unwrap_or(&POOL), value, shutdown).await;
}

pub async fn provide_instant<T>()
where
    T: InstantObject,
{
    provide_instant_impl::<T>(&POOL).await;
}

pub async fn provide_instant_context<T>()
where
    T: InstantObject,
{
    let context = context_pool();
    provide_instant_impl::<T>(context.as_deref().unwrap_or(&POOL)).await;
}

pub(crate) fn convert_shutdown_fn<T, F>(shutdown: F) -> ShutdownFn<T>
where
    T: Any + Send + Sync,
    F: ShutdownCallback<T>,
{
    Box::new(move |value: Arc<T>| {
        Box::pin(shutdown.call(value)) as Pin<Box<dyn Future<Output = ()> + Send>>
    })
}

/// Provide a value to the pool, it will let the old value shutdown if exists.
async fn provide_arc<T>(pool: &ObjectPool, value: Arc<T>, shutdown: Option<ShutdownFn<T>>)
where
    T: Any + Send + Sync,
{
    pool.set_value(value, shutdown).await;
}

async fn provide_arc_with<T, F>(pool: &ObjectPool, value: T, shutdown: F)
where
    T: Any + Send + Sync,
    F: ShutdownCallback<T>,
{
    provide_arc(pool, Arc::new(value), Some(convert_shutdown_fn(shutdown))).await;
}

pub async fn drop_value<T: Any + Send + Sync>() -> bool {
    POOL.remove_value::<T>().await
}

pub async fn drop_context_value<T: Any + Send + Sync>() -> bool {
    if let Some(pool) = context_pool() {
        return pool.remove_value::<T>().await;
    }

    false
}

pub async fn drop_entry<T: Any + Send + Sync>() -> bool {
    POOL.remove_entry::<T>().await
}

pub async fn drop_context_entry<T: Any + Send + Sync>() -> bool {
    if let Some(pool) = context_pool() {
        return pool.remove_entry::<T>().await;
    }

    false
}

/// Acquire a value from the pool, if not exists, wait until it is provided.
/// If the value is shutdown while waiting, return error immediately to avoid waiting forever.
/// This function is designed for the scenario that comsumer want to ensure a value is provided and blocking on it.
/// If you want to use it as a subscriber, you may got some value lost in the scenario that the value is frequently provided and comsumer is slow.
#[cfg(feature = "notify")]
pub async fn acquire<T: Any + Send + Sync>() -> Result<Arc<T>> {
    let context = context_pool();
    context.as_deref().unwrap_or(&POOL).wait_value::<T>().await;
    require::<T>().await
}

pub async fn fulfill<T: Any + Send + Sync>(value: T) -> Result<bool> {
    fulfill_arc(&POOL, Arc::new(value), None).await
}

pub async fn fulfill_with_shutdown<T, F>(value: T, shutdown: F) -> Result<bool>
where
    T: Any + Send + Sync,
    F: ShutdownCallback<T>,
{
    fulfill_arc_with(&POOL, value, shutdown).await
}

pub async fn fulfill_context<T: Any + Send + Sync>(value: T) -> Result<bool> {
    let context = context_pool();
    fulfill_arc(context.as_deref().unwrap_or(&POOL), Arc::new(value), None).await
}

pub async fn fulfill_with_context_shutdown<T, F>(value: T, shutdown: F) -> Result<bool>
where
    T: Any + Send + Sync,
    F: ShutdownCallback<T>,
{
    let context = context_pool();
    fulfill_arc_with(context.as_deref().unwrap_or(&POOL), value, shutdown).await
}

async fn fulfill_arc<T: Any + Send + Sync>(
    pool: &ObjectPool,
    value: Arc<T>,
    shutdown: Option<ShutdownFn<T>>,
) -> Result<bool> {
    pool.fulfill_value(value, shutdown).await
}

async fn fulfill_arc_with<T, F>(pool: &ObjectPool, value: T, shutdown: F) -> Result<bool>
where
    T: Any + Send + Sync,
    F: ShutdownCallback<T>,
{
    fulfill_arc(pool, Arc::new(value), Some(convert_shutdown_fn(shutdown))).await
}

pub async fn shutdown() {
    POOL.shutdown().await;
}

pub async fn shutdown_local() -> bool {
    if let Some(pool) = context_pool() {
        pool.shutdown().await;
        return true;
    }

    false
}

fn context_pool() -> Option<Arc<ObjectPool>> {
    CONTEXT.try_with(|pool| pool.clone()).ok()
}

async fn provide_instant_impl<T>(pool: &ObjectPool)
where
    T: InstantObject,
{
    let instant_value = T::create().await;
    let shutdown = |value: Arc<T::Output>| async move {
        T::destroy(value).await;
    };

    provide_arc(
        pool,
        Arc::new(instant_value),
        Some(convert_shutdown_fn(shutdown)),
    )
    .await;
}
