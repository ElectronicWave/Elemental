use elemental_core::http::SwappableClient;
use std::future::Future;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{broadcast, watch};

use super::event::{DownloadEvent, SessionEventStream, SessionSnapshot};
use super::limits::{BandwidthLimiter, ConcurrencyController};
use super::materializer::Materializer;
use super::report::{DownloadProgress, DownloadReport};
use super::task::{DownloadBatch, SessionId};
use crate::error::{Error, Result};

mod builder;
mod handler;
mod options;

pub use builder::DownloadSessionBuilder;
pub(crate) use handler::SessionHandler;
pub(crate) use options::DownloadSessionOptions;

#[derive(Debug)]
pub(super) struct SessionRuntime {
    pub(super) client: SwappableClient,
    pub(super) events: broadcast::Sender<DownloadEvent>,
    pub(super) concurrency: Arc<ConcurrencyController>,
    pub(super) bandwidth_limiter: Arc<BandwidthLimiter>,
    pub(super) materializer: Arc<dyn Materializer>,
    pub(super) default_session_parallelism: usize,
    pub(super) session_queue_capacity: usize,
}

#[derive(Debug, Clone)]
pub struct DownloadSession {
    lease: Arc<SessionLease>,
    name: Option<String>,
    progress: watch::Receiver<DownloadProgress>,
}

#[derive(Debug)]
struct SessionLease {
    id: SessionId,
    handler: Arc<SessionHandler>,
    closing: AtomicBool,
    closed: AtomicBool,
}

impl Drop for SessionLease {
    fn drop(&mut self) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }

        self.handler.abort_and_close_detached();
    }
}

impl DownloadSession {
    pub(super) fn new(
        id: SessionId,
        name: Option<String>,
        handler: Arc<SessionHandler>,
        progress: watch::Receiver<DownloadProgress>,
    ) -> Self {
        Self {
            lease: Arc::new(SessionLease {
                id,
                handler,
                closing: AtomicBool::new(false),
                closed: AtomicBool::new(false),
            }),
            name,
            progress,
        }
    }

    pub fn id(&self) -> SessionId {
        self.lease.id
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub async fn submit(&self, batch: impl Into<DownloadBatch>) -> Result<()> {
        let batch = batch.into();
        if batch.is_empty() {
            return Ok(());
        }
        if self.lease.closing.load(Ordering::Acquire) {
            return Err(Error::SessionNotFound {
                session_id: self.lease.id,
            });
        }
        self.lease.handler.submit_batch(batch).await
    }

    pub fn progress(&self) -> DownloadProgress {
        self.progress.borrow().clone()
    }

    pub fn progress_receiver(&self) -> watch::Receiver<DownloadProgress> {
        self.progress.clone()
    }

    pub fn subscribe(&self) -> SessionEventStream {
        SessionEventStream::new(self.lease.handler.subscribe_events(), self.lease.id)
    }

    pub async fn snapshot(&self) -> Result<SessionSnapshot> {
        if self.lease.closing.load(Ordering::Acquire) {
            return Err(Error::SessionNotFound {
                session_id: self.lease.id,
            });
        }
        Ok(self.lease.handler.snapshot())
    }

    pub async fn wait_idle(&self) -> Result<()> {
        if self.lease.closing.load(Ordering::Acquire) {
            return Err(Error::SessionNotFound {
                session_id: self.lease.id,
            });
        }
        self.lease.handler.wait_idle().await;
        Ok(())
    }

    /// Wait for all submitted tasks and return an error on failure or cancellation.
    ///
    /// The session stays open, so the caller can submit more work afterwards.
    pub async fn success(&self) -> Result<()> {
        if self.lease.closing.load(Ordering::Acquire) {
            return Err(Error::SessionNotFound {
                session_id: self.lease.id,
            });
        }
        self.lease.handler.wait_and_require_success().await
    }

    pub async fn finish(self) -> Result<DownloadReport> {
        let handler = Arc::clone(&self.lease.handler);
        self.complete(async move { Ok(handler.finish().await) })
            .await
    }

    pub async fn cancel(self) -> Result<DownloadReport> {
        let handler = Arc::clone(&self.lease.handler);
        self.complete(async move { Ok(handler.cancel().await) })
            .await
    }

    async fn complete(
        self,
        operation: impl Future<Output = Result<DownloadReport>>,
    ) -> Result<DownloadReport> {
        if self.lease.closing.swap(true, Ordering::AcqRel) {
            return Err(Error::SessionNotFound {
                session_id: self.lease.id,
            });
        }
        let report = operation.await?;
        self.lease.closed.store(true, Ordering::Release);
        Ok(report)
    }
}
