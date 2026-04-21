use std::{
    any::{Any, type_name},
    future::Future,
    pin::Pin,
    sync::Arc,
};

use anyhow::{Context, Result};

pub use crate::context::ObjectContext;
use crate::pool::{ObjectPool, POOL, ShutdownFn};
use crate::{context::CONTEXT, instant::InstantObject};

/// Fast try to get a value from the pool, if not exists, return error immediately.
pub async fn require<T: Any + Send + Sync>() -> Result<Arc<T>> {
    if let Some(pool) = context_pool()
        && let Ok(value) = pool.get_async::<T>().await.context(format!(
            "Cannot get object `{}` from pool",
            type_name::<T>()
        ))
    {
        return Ok(value);
    }

    POOL.get_async::<T>().await.context(format!(
        "Cannot get object `{}` from pool",
        type_name::<T>()
    ))
}

/// Provide a value to the pool, it will let the old value shutdown if exists.
/// Provide value to a existing value means safe **HOT RELOAD**
pub async fn provide<T>(value: T)
where
    T: Any + Send + Sync,
{
    provide_arc::<T>(Arc::new(value), None, false).await;
}

pub async fn provide_with_shutdown<T, F, Fut>(value: T, shutdown: F)
where
    T: Any + Send + Sync,
    F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    provide_arc(Arc::new(value), Some(convert_shutdown_fn(shutdown)), false).await;
}

pub async fn provide_context<T>(value: T)
where
    T: Any + Send + Sync,
{
    provide_arc(Arc::new(value), None, true).await;
}

pub async fn provide_context_with_shutdown<T, F, Fut>(value: T, shutdown: F)
where
    T: Any + Send + Sync,
    F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    provide_arc(Arc::new(value), Some(convert_shutdown_fn(shutdown)), true).await;
}

pub async fn provide_instant<T>()
where
    T: InstantObject,
{
    provide_instant_impl::<T>(false).await;
}

pub async fn provide_instant_context<T>()
where
    T: InstantObject,
{
    provide_instant_impl::<T>(true).await;
}

fn convert_shutdown_fn<T, F, Fut>(shutdown: F) -> ShutdownFn<T>
where
    T: Any + Send + Sync,
    F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    Box::new(move |value: Arc<T>| {
        Box::pin(shutdown(value)) as Pin<Box<dyn Future<Output = ()> + Send>>
    })
}
/// Provide a value to the pool, it will let the old value shutdown if exists.
async fn provide_arc<T>(value: Arc<T>, shutdown: Option<ShutdownFn<T>>, context: bool)
where
    T: Any + Send + Sync,
{
    // Try to set value in context first, if context exists, otherwise set it in global pool.
    if let Some(pool) = selected_context_pool(context) {
        pool.set_value(value, shutdown).await;
        return;
    }

    POOL.set_value(value, shutdown).await;
}

pub async fn drop_value<T: Any + Send + Sync>() {
    POOL.remove_value::<T>().await;
}

pub async fn drop_context_value<T: Any + Send + Sync>() {
    if let Some(pool) = context_pool() {
        pool.remove_value::<T>().await;
    }
}

pub async fn drop_entry<T: Any + Send + Sync>() {
    POOL.remove_entry::<T>().await;
}

pub async fn drop_context_entry<T: Any + Send + Sync>() {
    if let Some(pool) = context_pool() {
        pool.remove_entry::<T>().await;
    }
}

/// Acquire a value from the pool, if not exists, wait until it is provided.
/// If the value is shutdown while waiting, return error immediately to avoid waiting forever.
/// This function is designed for the scenario that comsumer want to ensure a value is provided and blocking on it.
/// If you want to use it as a subscriber, you may got some value lost in the scenario that the value is frequently provided and comsumer is slow.
#[cfg(feature = "notify")]
pub async fn acquire<T: Any + Send + Sync>() -> Result<Arc<T>> {
    // If context exists, wait value in context, otherwise wait value in global pool.
    if let Some(pool) = context_pool() {
        pool.wait_value::<T>().await;
    } else {
        POOL.wait_value::<T>().await;
    }
    require::<T>().await
}

// Who first provide the value, we use whose shutdown function, even if the value is overwritten later, so the shutdown function can always access the value.
pub async fn fulfill<T: Any + Send + Sync>(value: T) {
    fulfill_arc(Arc::new(value), None, false).await;
}

pub async fn fulfill_with_shutdown<T, F, Fut>(value: T, shutdown: F)
where
    T: Any + Send + Sync,
    F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    fulfill_arc(Arc::new(value), Some(convert_shutdown_fn(shutdown)), false).await;
}

pub async fn fulfill_context<T: Any + Send + Sync>(value: T) {
    fulfill_arc(Arc::new(value), None, true).await;
}

pub async fn fulfill_with_context_shutdown<T, F, Fut>(value: T, shutdown: F)
where
    T: Any + Send + Sync,
    F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    fulfill_arc(Arc::new(value), Some(convert_shutdown_fn(shutdown)), true).await;
}

async fn fulfill_arc<T: Any + Send + Sync>(
    value: Arc<T>,
    shutdown: Option<ShutdownFn<T>>,
    context: bool,
) {
    if let Some(pool) = selected_context_pool(context) {
        pool.fulfill_value(value, shutdown).await;
        return;
    }
    POOL.fulfill_value(value, shutdown).await;
}

pub async fn shutdown() {
    POOL.shutdown().await;
}

pub async fn shutdown_local() {
    if let Some(pool) = context_pool() {
        pool.shutdown().await;
    }
}

fn context_pool() -> Option<Arc<ObjectPool>> {
    CONTEXT.try_with(|pool| pool.clone()).ok()
}

fn selected_context_pool(context: bool) -> Option<Arc<ObjectPool>> {
    if context {
        return context_pool();
    }

    None
}

async fn provide_instant_impl<T>(context: bool)
where
    T: InstantObject,
{
    let instant_value = T::create().await;
    let shutdown = |value: Arc<T::Output>| async move {
        T::destroy(value).await;
    };

    provide_arc(
        Arc::new(instant_value),
        Some(convert_shutdown_fn(shutdown)),
        context,
    )
    .await;
}
