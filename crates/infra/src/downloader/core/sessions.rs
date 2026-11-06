use std::sync::{Arc, atomic::Ordering};
use tokio::sync::{Mutex as AsyncMutex, mpsc};

use super::ElementalDownloader;
use crate::downloader::batch::DownloadBatch;
use crate::downloader::event::{DownloadEvent, SessionSnapshot};
use crate::downloader::limits::BandwidthLimiter;
use crate::downloader::report::DownloadReport;
use crate::downloader::session::{DownloadSession, DownloadSessionOptions, SessionHandler};
use crate::downloader::task::SessionId;
use crate::downloader::worker::run_session_worker;
use crate::error::{Error, Result};

impl ElementalDownloader {
    pub(crate) async fn create_session(
        &self,
        name: Option<String>,
        options: DownloadSessionOptions,
    ) -> Result<DownloadSession> {
        options.validate()?;
        let session_id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
        let downloader = self.me.upgrade().ok_or(Error::DownloaderDropped)?;
        let (sender, receiver) = mpsc::channel(self.session_queue_capacity);
        let receiver = Arc::new(AsyncMutex::new(receiver));
        let parallelism = options.effective_parallelism(self.default_session_parallelism);
        let session_rate_limit = options.effective_rate_limit();
        let (handler, progress) = SessionHandler::new(
            session_id,
            name.clone(),
            options,
            sender,
            Arc::new(BandwidthLimiter::new(session_rate_limit)?),
            self.events.clone(),
        );
        let handler = Arc::new(handler);

        for _ in 0..parallelism {
            let handler_for_worker = handler.clone();
            let receiver_for_worker = receiver.clone();
            let downloader_for_worker = downloader.clone();
            handler.spawn_worker(async move {
                run_session_worker(
                    downloader_for_worker,
                    handler_for_worker,
                    receiver_for_worker,
                )
                .await;
            });
        }

        let handler_for_event = handler.clone();
        self.sessions
            .insert_async(session_id, handler)
            .await
            .map_err(|_| Error::SessionIdConflict { session_id })?;
        handler_for_event.emit(DownloadEvent::SessionOpened {
            session_id,
            session_name: name.clone(),
        });

        Ok(DownloadSession::new(
            session_id,
            name,
            self.me.clone(),
            progress,
            self.events.clone(),
        ))
    }

    pub(crate) async fn submit(&self, session_id: SessionId, batch: DownloadBatch) -> Result<()> {
        let (group, tasks) = batch.into_parts();
        if tasks.is_empty() {
            return Ok(());
        }

        for task in &tasks {
            task.validate()?;
        }

        let handler = self.session_handler(session_id).await?;
        let _submission_guard = handler.submission_guard().await;

        handler.ensure_accepting_tasks(session_id)?;

        for task in tasks {
            handler.enqueue_task(task, group.clone()).await?;
        }

        Ok(())
    }

    pub(crate) async fn wait_session_idle(&self, session_id: SessionId) -> Result<()> {
        self.session_handler(session_id).await?.wait_idle().await;
        Ok(())
    }

    pub(crate) async fn session_snapshot(&self, session_id: SessionId) -> Result<SessionSnapshot> {
        Ok(self.session_handler(session_id).await?.snapshot())
    }

    pub(crate) async fn session_success(&self, session_id: SessionId) -> Result<()> {
        let handler = self.session_handler(session_id).await?;
        let _submission_guard = handler.submission_guard().await;
        handler.wait_idle().await;
        handler.build_report().success()
    }

    pub(crate) async fn finish_session_report(
        &self,
        session_id: SessionId,
    ) -> Result<DownloadReport> {
        Ok(self.take_session(session_id).await?.finish().await)
    }

    pub(crate) async fn cancel_session_report(
        &self,
        session_id: SessionId,
    ) -> Result<DownloadReport> {
        Ok(self.take_session(session_id).await?.cancel().await)
    }

    pub(crate) fn abort_session_detached(&self, session_id: SessionId) {
        let Some((_, handler)) = self.sessions.remove_sync(&session_id) else {
            return;
        };

        handler.abort_detached();
        let report_handler = handler.clone();
        let wait_handler = handler.clone();
        let close = async move {
            wait_handler.wait_workers().await;
            report_handler.emit_closed_report();
        };

        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.spawn(close);
            }
            Err(_) => {
                handler.emit_closed_report();
            }
        }
    }

    async fn session_handler(&self, session_id: SessionId) -> Result<Arc<SessionHandler>> {
        self.sessions
            .read_async(&session_id, |_, handler| handler.clone())
            .await
            .ok_or(Error::SessionNotFound { session_id })
    }

    async fn take_session(&self, session_id: SessionId) -> Result<Arc<SessionHandler>> {
        self.sessions
            .remove_async(&session_id)
            .await
            .map(|(_, handler)| handler)
            .ok_or(Error::SessionNotFound { session_id })
    }
}
