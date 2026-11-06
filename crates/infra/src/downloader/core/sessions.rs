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
        if let Some(old) = self.sessions.upsert_async(session_id, handler).await {
            old.close().await;
        }
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

        if handler.is_closed() {
            return Err(Error::SessionClosed { session_id });
        }

        if handler.is_scheduling_stopped() {
            return Err(Error::SessionNotAcceptingTasks { session_id });
        }

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
        let handler = self.session_handler(session_id).await?;
        handler.close_and_wait().await;
        self.remove_session(session_id).await
    }

    pub(crate) async fn cancel_session_report(
        &self,
        session_id: SessionId,
    ) -> Result<DownloadReport> {
        let handler = self.session_handler(session_id).await?;
        handler.cancel();
        handler.close_and_wait().await;
        self.remove_session(session_id).await
    }

    async fn remove_session(&self, session_id: SessionId) -> Result<DownloadReport> {
        let handler = self.session_handler(session_id).await?;
        let report = handler.build_report();
        handler.emit(DownloadEvent::SessionClosed {
            session_id,
            report: report.clone(),
        });
        self.sessions.remove_async(&session_id).await;
        Ok(report)
    }

    pub(crate) fn abort_session_detached(&self, session_id: SessionId) {
        let Some((_, handler)) = self.sessions.remove_sync(&session_id) else {
            return;
        };

        handler.abort_detached();
        let events = self.events.clone();
        let report_handler = handler.clone();
        let wait_handler = handler.clone();
        let close = async move {
            wait_handler.wait_workers().await;
            let _ = events.send(DownloadEvent::SessionClosed {
                session_id,
                report: report_handler.build_report(),
            });
        };

        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.spawn(close);
            }
            Err(_) => {
                handler.emit(DownloadEvent::SessionClosed {
                    session_id,
                    report: handler.build_report(),
                });
            }
        }
    }

    async fn session_handler(&self, session_id: SessionId) -> Result<Arc<SessionHandler>> {
        self.sessions
            .read_async(&session_id, |_, handler| handler.clone())
            .await
            .ok_or(Error::SessionNotFound { session_id })
    }
}
