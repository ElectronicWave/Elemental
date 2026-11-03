pub mod context;
pub mod error;
pub mod facade;
pub mod instant;
pub mod pool;
pub use error::{Error, Result};
pub use facade::*;

#[cfg(test)]
mod testobj {
    use super::*;
    use crate::facade::convert_shutdown_fn;
    use crate::pool::ObjectPool;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };
    use tokio::{
        sync::Notify,
        time::{sleep, timeout},
    };

    #[tokio::test]
    async fn test_provide_and_remove() {
        let pool = ObjectPool::new();
        let shutdown_count = Arc::new(AtomicUsize::new(0));
        let count = shutdown_count.clone();

        pool.set_value(
            Arc::new("value".to_owned()),
            Some(convert_shutdown_fn(move |_value: Arc<String>| {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                }
            })),
        )
        .await;

        let value = pool
            .get_async::<String>()
            .await
            .expect("value should exist");
        assert_eq!(value.as_str(), "value");

        assert!(pool.remove_value::<String>().await);
        assert_eq!(shutdown_count.load(Ordering::SeqCst), 1);
        assert!(pool.get_async::<String>().await.is_none());
        assert!(!pool.remove_value::<String>().await);
    }

    #[cfg(feature = "notify")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wait_for_provision() {
        let pool = Arc::new(ObjectPool::new());
        let waiter = {
            let pool = pool.clone();
            tokio::spawn(async move {
                pool.wait_value::<usize>().await;
                pool.get_async::<usize>().await
            })
        };

        sleep(Duration::from_millis(50)).await;
        assert!(!waiter.is_finished());

        pool.set_value(Arc::new(7usize), None).await;
        let value = timeout(Duration::from_secs(1), waiter)
            .await
            .expect("waiter should be notified")
            .expect("waiter task should not panic")
            .expect("value should exist");
        assert_eq!(*value, 7);
    }

    #[cfg(feature = "notify")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_fulfill_requires_pending_entry() {
        let pool = Arc::new(ObjectPool::new());
        assert!(pool.fulfill_value(Arc::new(7usize), None).await.is_err());

        let waiter = {
            let pool = pool.clone();
            tokio::spawn(async move {
                pool.wait_value::<usize>().await;
                pool.get_async::<usize>().await
            })
        };

        let mut filled = false;
        for _ in 0..100 {
            match pool.fulfill_value(Arc::new(7usize), None).await {
                Ok(true) => {
                    filled = true;
                    break;
                }
                Ok(false) => panic!("unexpected existing value"),
                Err(_) => sleep(Duration::from_millis(10)).await,
            }
        }
        assert!(filled, "waiter did not create a pending entry");

        let value = timeout(Duration::from_secs(1), waiter)
            .await
            .expect("waiter should be notified")
            .expect("waiter task should not panic")
            .expect("value should exist");
        assert_eq!(*value, 7);

        assert!(
            !pool
                .fulfill_value(Arc::new(8usize), None)
                .await
                .expect("already filled should be reported")
        );
        let value = pool
            .get_async::<usize>()
            .await
            .expect("value should remain");
        assert_eq!(*value, 7);
    }

    #[cfg(feature = "notify")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_drop_value_closes_slot_and_drop_entry_removes_it() {
        let pool = Arc::new(ObjectPool::new());
        pool.set_value(Arc::new(1usize), None).await;

        assert!(pool.remove_value::<usize>().await);
        timeout(Duration::from_millis(100), pool.wait_value::<usize>())
            .await
            .expect("closed entry must not block");
        assert!(pool.get_async::<usize>().await.is_none());

        pool.set_value(Arc::new(2usize), None).await;
        assert!(pool.remove_entry::<usize>().await);
        assert!(
            timeout(Duration::from_millis(50), pool.wait_value::<usize>())
                .await
                .is_err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_replacement_runs_old_shutdown_outside_entry_lock() {
        let pool = Arc::new(ObjectPool::new());
        let shutdown_started = Arc::new(Notify::new());
        let shutdown_release = Arc::new(Notify::new());
        let started = shutdown_started.clone();
        let release = shutdown_release.clone();

        pool.set_value(
            Arc::new(1i32),
            Some(convert_shutdown_fn(move |_value: Arc<i32>| {
                let started = started.clone();
                let release = release.clone();
                async move {
                    started.notify_one();
                    release.notified().await;
                }
            })),
        )
        .await;

        let replacement = {
            let pool = pool.clone();
            tokio::spawn(async move {
                pool.set_value(Arc::new(2i32), None).await;
            })
        };

        timeout(Duration::from_secs(1), shutdown_started.notified())
            .await
            .expect("old shutdown should start");

        let value = timeout(Duration::from_millis(200), pool.get_async::<i32>())
            .await
            .expect("read must not be blocked by old shutdown")
            .expect("new value should be visible");
        assert_eq!(*value, 2);

        shutdown_release.notify_one();
        timeout(Duration::from_secs(1), replacement)
            .await
            .expect("replacement should finish")
            .expect("replacement task should not panic");
    }

    #[derive(Debug, PartialEq)]
    struct ContextValue(String);

    #[cfg(feature = "notify")]
    #[tokio::test]
    async fn test_context_acquire() {
        let context = ObjectContext::new();
        let value = context
            .clone()
            .run(async {
                let consumer = async { acquire::<ContextValue>().await };
                let producer = async {
                    sleep(Duration::from_millis(50)).await;
                    provide_context(ContextValue("context".to_owned())).await;
                };
                let (value, ()) = tokio::join!(consumer, producer);
                value
            })
            .await
            .expect("context value should be acquired");

        assert_eq!(value.0, "context");
        context.shutdown().await;
    }

    #[derive(Debug, PartialEq)]
    struct GlobalValue(usize);

    #[tokio::test]
    async fn test_facade_provide_require_drop() {
        provide(GlobalValue(1)).await;
        assert_eq!(
            require::<GlobalValue>()
                .await
                .expect("value should exist")
                .0,
            1
        );

        assert!(drop_value::<GlobalValue>().await);
        assert!(require::<GlobalValue>().await.is_err());
    }
}
