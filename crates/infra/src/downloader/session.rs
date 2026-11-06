use std::future::Future;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::watch;

use super::batch::DownloadBatch;
use super::core::ElementalDownloader;
use super::event::{SessionEventStream, SessionSnapshot};
use super::report::{DownloadProgress, DownloadReport};
use super::task::SessionId;
use crate::error::Result;

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
}

#[derive(Debug)]
struct SessionLease {
    id: SessionId,
    downloader: Arc<ElementalDownloader>,
    finished: AtomicBool,
}

impl Drop for SessionLease {
    fn drop(&mut self) {
        if self.finished.swap(true, Ordering::AcqRel) {
            return;
        }

        self.downloader.abort_session_detached(self.id);
    }
}

impl DownloadSession {
    pub(super) fn new(
        id: SessionId,
        name: Option<String>,
        downloader: Arc<ElementalDownloader>,
        progress: watch::Receiver<DownloadProgress>,
    ) -> Self {
        Self {
            lease: Arc::new(SessionLease {
                id,
                downloader,
                finished: AtomicBool::new(false),
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
        self.lease
            .downloader
            .submit(self.lease.id, batch.into())
            .await
    }

    pub fn progress(&self) -> DownloadProgress {
        self.progress.borrow().clone()
    }

    pub fn progress_receiver(&self) -> watch::Receiver<DownloadProgress> {
        self.progress.clone()
    }

    pub fn subscribe(&self) -> SessionEventStream {
        SessionEventStream::new(self.lease.downloader.event_receiver(), self.lease.id)
    }

    pub async fn snapshot(&self) -> Result<SessionSnapshot> {
        self.lease.downloader.session_snapshot(self.lease.id).await
    }

    pub async fn wait_idle(&self) -> Result<()> {
        self.lease.downloader.wait_session_idle(self.lease.id).await
    }

    pub async fn success(&self) -> Result<()> {
        self.lease.downloader.session_success(self.lease.id).await
    }

    pub async fn finish(self) -> Result<DownloadReport> {
        let session_id = self.lease.id;
        let downloader = Arc::clone(&self.lease.downloader);
        self.complete(downloader.finish_session_report(session_id))
            .await
    }

    pub async fn cancel(self) -> Result<DownloadReport> {
        let session_id = self.lease.id;
        let downloader = Arc::clone(&self.lease.downloader);
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
