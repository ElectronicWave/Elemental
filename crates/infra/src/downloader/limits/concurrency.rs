use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{Mutex as AsyncMutex, OwnedSemaphorePermit, Semaphore};

use crate::error::{Error, Result};

#[derive(Debug)]
pub(crate) struct ConcurrencyController {
    semaphore: Arc<Semaphore>,
    target: AtomicUsize,
    resize: AsyncMutex<()>,
    held_permits: AsyncMutex<Vec<OwnedSemaphorePermit>>,
}

impl ConcurrencyController {
    pub(crate) fn new(max_concurrent_downloads: usize) -> Result<Self> {
        if max_concurrent_downloads == 0 {
            return Err(Error::InvalidMaxConcurrentDownloads);
        }

        Ok(Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent_downloads)),
            target: AtomicUsize::new(max_concurrent_downloads),
            resize: AsyncMutex::new(()),
            held_permits: AsyncMutex::new(Vec::new()),
        })
    }

    pub(crate) fn max_concurrent_downloads(&self) -> usize {
        self.target.load(Ordering::Acquire)
    }

    pub(crate) async fn set_max_concurrent_downloads(
        &self,
        max_concurrent_downloads: usize,
    ) -> Result<()> {
        if max_concurrent_downloads == 0 {
            return Err(Error::InvalidMaxConcurrentDownloads);
        }

        let _resize_guard = self.resize.lock().await;
        let current = self.target.load(Ordering::Acquire);
        if current == max_concurrent_downloads {
            return Ok(());
        }

        if max_concurrent_downloads > current {
            let mut held_permits = self.held_permits.lock().await;
            let delta = max_concurrent_downloads - current;
            let released = delta.min(held_permits.len());

            for _ in 0..released {
                held_permits.pop();
            }

            let remaining = delta - released;
            if remaining > 0 {
                self.semaphore.add_permits(remaining);
            }

            self.target
                .store(max_concurrent_downloads, Ordering::Release);
            return Ok(());
        }

        let delta = current - max_concurrent_downloads;
        let mut held_permits = self.held_permits.lock().await;
        for _ in 0..delta {
            let permit = self
                .semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(Error::ConcurrencyShrink)?;
            held_permits.push(permit);
        }

        self.target
            .store(max_concurrent_downloads, Ordering::Release);
        Ok(())
    }

    pub(crate) async fn acquire_owned(&self) -> Result<OwnedSemaphorePermit> {
        self.semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(Error::ConcurrencyAcquire)
    }
}
