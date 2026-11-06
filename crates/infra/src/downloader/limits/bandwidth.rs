use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
use tokio::sync::Mutex as AsyncMutex;

use crate::downloader::rate::{ByteRate, DownloadRateLimit};
use crate::error::Result;

#[derive(Debug)]
pub(crate) struct BandwidthLimiter {
    bytes_per_second: AtomicUsize,
    state: AsyncMutex<BandwidthWindow>,
}

#[derive(Debug)]
struct BandwidthWindow {
    tokens: f64,
    last_refill: Instant,
}

impl BandwidthLimiter {
    pub(crate) fn new(rate_limit: DownloadRateLimit) -> Result<Self> {
        let bytes_per_second = Self::encode_rate_limit(rate_limit);
        Ok(Self {
            bytes_per_second: AtomicUsize::new(bytes_per_second),
            state: AsyncMutex::new(BandwidthWindow {
                tokens: bytes_per_second as f64,
                last_refill: Instant::now(),
            }),
        })
    }

    pub(crate) fn rate_limit(&self) -> DownloadRateLimit {
        Self::decode_rate_limit(self.bytes_per_second.load(Ordering::Acquire))
    }

    pub(crate) async fn set_rate_limit(&self, rate_limit: DownloadRateLimit) -> Result<()> {
        let bytes_per_second = Self::encode_rate_limit(rate_limit);
        self.bytes_per_second
            .store(bytes_per_second, Ordering::Release);

        let mut state = self.state.lock().await;
        state.tokens = bytes_per_second as f64;
        state.last_refill = Instant::now();
        Ok(())
    }

    pub(crate) async fn throttle(&self, bytes: usize) {
        if bytes == 0 {
            return;
        }

        let bytes_per_second = self.bytes_per_second.load(Ordering::Acquire);
        if bytes_per_second == 0 {
            return;
        }

        let delay = {
            let mut state = self.state.lock().await;
            let now = Instant::now();
            let rate = bytes_per_second as f64;
            let elapsed = now.saturating_duration_since(state.last_refill);
            state.tokens = (state.tokens + elapsed.as_secs_f64() * rate).min(rate);
            state.last_refill = now;

            let needed = bytes as f64;
            let delay = if state.tokens >= needed {
                None
            } else {
                Some(Duration::from_secs_f64((needed - state.tokens) / rate))
            };

            // Allow the balance to go negative: a chunk larger than the burst
            // consumes its full cost up front, and later chunks inherit the debt.
            state.tokens -= needed;
            delay
        };

        if let Some(delay) = delay {
            tokio::time::sleep(delay).await;
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

        DownloadRateLimit::Limited(ByteRate::bytes_per_sec(
            std::num::NonZeroUsize::new(bytes_per_second)
                .expect("stored downloader rate limit must be non-zero"),
        ))
    }
}
