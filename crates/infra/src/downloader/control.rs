use anyhow::{Context, Result, bail};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{Mutex as AsyncMutex, OwnedSemaphorePermit, Semaphore};

use super::task::DownloadRateLimit;

#[derive(Debug)]
pub(crate) struct ConcurrencyController {
    semaphore: Arc<Semaphore>,
    target: AtomicUsize,
    resize: AsyncMutex<()>,
    held_permits: AsyncMutex<Vec<OwnedSemaphorePermit>>,
}

impl ConcurrencyController {
    pub(crate) fn new(max_connections: usize) -> Result<Self> {
        if max_connections == 0 {
            bail!("downloader max_connections must be greater than zero");
        }

        Ok(Self {
            semaphore: Arc::new(Semaphore::new(max_connections)),
            target: AtomicUsize::new(max_connections),
            resize: AsyncMutex::new(()),
            held_permits: AsyncMutex::new(Vec::new()),
        })
    }

    pub(crate) fn max_connections(&self) -> usize {
        self.target.load(Ordering::Acquire)
    }

    pub(crate) async fn set_max_connections(&self, max_connections: usize) -> Result<()> {
        if max_connections == 0 {
            bail!("downloader max_connections must be greater than zero");
        }

        let _resize_guard = self.resize.lock().await;
        let current = self.target.load(Ordering::Acquire);
        if current == max_connections {
            return Ok(());
        }

        if max_connections > current {
            let mut held_permits = self.held_permits.lock().await;
            let delta = max_connections - current;
            let released = delta.min(held_permits.len());

            for _ in 0..released {
                held_permits.pop();
            }

            let remaining = delta - released;
            if remaining > 0 {
                self.semaphore.add_permits(remaining);
            }

            self.target.store(max_connections, Ordering::Release);
            return Ok(());
        }

        let delta = current - max_connections;
        let mut held_permits = self.held_permits.lock().await;
        for _ in 0..delta {
            let permit = self
                .semaphore
                .clone()
                .acquire_owned()
                .await
                .context("failed to shrink downloader concurrency")?;
            held_permits.push(permit);
        }

        self.target.store(max_connections, Ordering::Release);
        Ok(())
    }

    pub(crate) async fn acquire_owned(&self) -> Result<OwnedSemaphorePermit> {
        self.semaphore
            .clone()
            .acquire_owned()
            .await
            .context("failed to acquire downloader concurrency permit")
    }
}

#[derive(Debug)]
pub(crate) struct BandwidthLimiter {
    bytes_per_second: AtomicUsize,
    state: AsyncMutex<BandwidthWindow>,
}

#[derive(Debug)]
struct BandwidthWindow {
    started_at: Instant,
    used_bytes: usize,
}

impl BandwidthLimiter {
    pub(crate) fn new(rate_limit: DownloadRateLimit) -> Result<Self> {
        Ok(Self {
            bytes_per_second: AtomicUsize::new(encode_rate_limit(rate_limit)),
            state: AsyncMutex::new(BandwidthWindow {
                started_at: Instant::now(),
                used_bytes: 0,
            }),
        })
    }

    pub(crate) fn rate_limit(&self) -> DownloadRateLimit {
        decode_rate_limit(self.bytes_per_second.load(Ordering::Acquire))
    }

    pub(crate) async fn set_rate_limit(&self, rate_limit: DownloadRateLimit) -> Result<()> {
        self.bytes_per_second
            .store(encode_rate_limit(rate_limit), Ordering::Release);

        let mut state = self.state.lock().await;
        state.started_at = Instant::now();
        state.used_bytes = 0;
        Ok(())
    }

    pub(crate) async fn throttle(&self, bytes: usize) {
        if bytes == 0 {
            return;
        }

        loop {
            let bytes_per_second = self.bytes_per_second.load(Ordering::Acquire);
            if bytes_per_second == 0 {
                return;
            }

            let delay = {
                let mut state = self.state.lock().await;
                let now = Instant::now();
                if now.duration_since(state.started_at) >= Duration::from_secs(1) {
                    state.started_at = now;
                    state.used_bytes = 0;
                }

                let cost = bytes.min(bytes_per_second);
                if state.used_bytes + cost <= bytes_per_second {
                    state.used_bytes += cost;
                    None
                } else if state.used_bytes == 0 {
                    state.used_bytes = cost;
                    None
                } else {
                    Some((state.started_at + Duration::from_secs(1)).saturating_duration_since(now))
                }
            };

            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
                continue;
            }

            return;
        }
    }
}

fn encode_rate_limit(rate_limit: DownloadRateLimit) -> usize {
    match rate_limit {
        DownloadRateLimit::Unlimited => 0,
        DownloadRateLimit::Limited(rate) => rate.as_bytes_per_sec(),
    }
}

fn decode_rate_limit(bytes_per_second: usize) -> DownloadRateLimit {
    if bytes_per_second == 0 {
        return DownloadRateLimit::Unlimited;
    }

    DownloadRateLimit::Limited(super::task::ByteRate::bytes_per_sec(
        std::num::NonZeroUsize::new(bytes_per_second)
            .expect("stored downloader rate limit must be non-zero"),
    ))
}
