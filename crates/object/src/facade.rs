use std::{
    any::{Any, type_name},
    future::Future,
    pin::Pin,
    sync::Arc,
};

use anyhow::{Context, Result};

pub use crate::context::ObjectContext;
use crate::pool::{POOL, ShutdownFn};
use crate::{context::CONTEXT, instant::InstantObject};
pub async fn require<T: Any + Send + Sync>() -> Result<Arc<T>> {
    let pool = CONTEXT.try_with(|pool| pool.clone());
    if let Ok(pool) = pool {
        if let Ok(value) = pool.get_async::<T>().await.context(format!(
            "Cannot get object `{}` from pool",
            type_name::<T>()
        )) {
            return Ok(value);
        }
    }

    POOL.get_async::<T>().await.context(format!(
        "Cannot get object `{}` from pool",
        type_name::<T>()
    ))
}

/// Provide a value to the pool, it will let the old value shutdown if exists.
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
    let instant_value = T::create().await;
    let shutdown = |value: Arc<T::Output>| async move {
        T::destroy(value).await;
    };
    provide_arc(
        Arc::new(instant_value),
        Some(convert_shutdown_fn(shutdown)),
        false,
    )
    .await;
}

pub async fn provide_instant_context<T>()
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
        true,
    )
    .await;
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
    if context {
        let pool = CONTEXT.try_with(|pool| pool.clone());
        if let Ok(pool) = pool {
            pool.set_value(value, shutdown).await;
            return;
        }
    }

    POOL.set_value(value, shutdown).await;
}

pub async fn drop_value<T: Any + Send + Sync>() {
    POOL.remove_value::<T>().await;
}

pub async fn drop_context_value<T: Any + Send + Sync>() {
    let pool = CONTEXT.try_with(|pool| pool.clone());
    if let Ok(pool) = pool {
        pool.remove_value::<T>().await;
    }
}

pub async fn drop_entry<T: Any + Send + Sync>() {
    POOL.remove_entry::<T>().await;
}

pub async fn drop_context_entry<T: Any + Send + Sync>() {
    let pool = CONTEXT.try_with(|pool| pool.clone());
    if let Ok(pool) = pool {
        pool.remove_entry::<T>().await;
    }
}

/// Acquire a value from the pool, if not exists, wait until it is provided.
#[cfg(feature = "notify")]
pub async fn acquire<T: Any + Send + Sync>() -> Result<Arc<T>> {
    let pool = CONTEXT.try_with(|pool| pool.clone());
    // If context exists, wait value in context, otherwise wait value in global pool.
    if let Ok(pool) = pool {
        pool.wait_value::<T>().await;
    } else {
        POOL.wait_value::<T>().await;
    }
    require::<T>().await
}

// Who first provide the value, we use whose shutdown function, even if the value is overwritten later, so the shutdown function can always access the value.
#[cfg(feature = "notify")]
pub async fn fulfill<T: Any + Send + Sync>(value: T) {
    fulfill_arc(Arc::new(value), None, false).await;
}

#[cfg(feature = "notify")]
pub async fn fulfill_with_shutdown<T, F, Fut>(value: T, shutdown: F)
where
    T: Any + Send + Sync,
    F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    fulfill_arc(Arc::new(value), Some(convert_shutdown_fn(shutdown)), false).await;
}

#[cfg(feature = "notify")]
pub async fn fulfill_context<T: Any + Send + Sync>(value: T) {
    fulfill_arc(Arc::new(value), None, true).await;
}

#[cfg(feature = "notify")]
pub async fn fulfill_with_context_shutdown<T, F, Fut>(value: T, shutdown: F)
where
    T: Any + Send + Sync,
    F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    fulfill_arc(Arc::new(value), Some(convert_shutdown_fn(shutdown)), true).await;
}

#[cfg(feature = "notify")]
async fn fulfill_arc<T: Any + Send + Sync>(
    value: Arc<T>,
    shutdown: Option<ShutdownFn<T>>,
    context: bool,
) {
    if context {
        let pool = CONTEXT.try_with(|pool| pool.clone());
        if let Ok(pool) = pool {
            pool.fulfill_value(value, shutdown).await;
            return;
        }
    }
    POOL.fulfill_value(value, shutdown).await;
}

pub async fn shutdown() {
    POOL.shutdown().await;
}

pub async fn shutdown_local() {
    let pool = CONTEXT.try_with(|pool| pool.clone());
    if let Ok(pool) = pool {
        pool.shutdown().await;
    }
}
