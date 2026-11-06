use std::future::Future;
use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{broadcast, watch};

use super::batch::DownloadBatch;
use super::core::ElementalDownloader;
use super::event::{DownloadEvent, SessionEventStream, SessionSnapshot};
use super::report::{DownloadProgress, DownloadReport};
use super::task::SessionId;
use crate::error::{Error, Result};

mod builder;
mod handler;
mod options;

pub use builder::DownloadSessionBuilder;
pub(crate) use handler::SessionHandler;
pub(crate) use options::DownloadSessionOptions;

#[derive(Debug, Clone)]
pub struct DownloadSession {
    lease: Arc<SessionLease>,
    name: Option<String>,
    progress: watch::Receiver<DownloadProgress>,
    events: broadcast::Sender<DownloadEvent>,
}

#[derive(Debug)]
struct SessionLease {
    id: SessionId,
    downloader: Weak<ElementalDownloader>,
    finished: AtomicBool,
}

impl Drop for SessionLease {
    fn drop(&mut self) {
        if self.finished.swap(true, Ordering::AcqRel) {
            return;
        }

        if let Some(downloader) = self.downloader.upgrade() {
            downloader.abort_session_detached(self.id);
        }
    }
}

impl DownloadSession {
    pub(super) fn new(
        id: SessionId,
        name: Option<String>,
        downloader: Weak<ElementalDownloader>,
        progress: watch::Receiver<DownloadProgress>,
        events: broadcast::Sender<DownloadEvent>,
    ) -> Self {
        Self {
            lease: Arc::new(SessionLease {
                id,
                downloader,
                finished: AtomicBool::new(false),
            }),
            name,
            progress,
            events,
        }
    }

    fn downloader(&self) -> Result<Arc<ElementalDownloader>> {
        self.lease
            .downloader
            .upgrade()
            .ok_or(Error::DownloaderDropped)
    }

    pub fn id(&self) -> SessionId {
        self.lease.id
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub async fn submit(&self, batch: impl Into<DownloadBatch>) -> Result<()> {
        self.downloader()?.submit(self.lease.id, batch.into()).await
    }

    pub fn progress(&self) -> DownloadProgress {
        self.progress.borrow().clone()
    }

    pub fn progress_receiver(&self) -> watch::Receiver<DownloadProgress> {
        self.progress.clone()
    }

    pub fn subscribe(&self) -> SessionEventStream {
        SessionEventStream::new(self.events.subscribe(), self.lease.id)
    }

    pub async fn snapshot(&self) -> Result<SessionSnapshot> {
        self.downloader()?.session_snapshot(self.lease.id).await
    }

    pub async fn wait_idle(&self) -> Result<()> {
        self.downloader()?.wait_session_idle(self.lease.id).await
    }

    pub async fn success(&self) -> Result<()> {
        self.downloader()?.session_success(self.lease.id).await
    }

    pub async fn finish(self) -> Result<DownloadReport> {
        let downloader = self.downloader()?;
        let session_id = self.lease.id;
        self.complete(downloader.finish_session_report(session_id))
            .await
    }

    pub async fn cancel(self) -> Result<DownloadReport> {
        let downloader = self.downloader()?;
        let session_id = self.lease.id;
        self.complete(downloader.cancel_session_report(session_id))
            .await
    }

    async fn complete(
        self,
        operation: impl Future<Output = Result<DownloadReport>>,
    ) -> Result<DownloadReport> {
        let result = operation.await;
        self.mark_finished();
        result
    }

    fn mark_finished(&self) {
        self.lease.finished.store(true, Ordering::Release);
    }
}
