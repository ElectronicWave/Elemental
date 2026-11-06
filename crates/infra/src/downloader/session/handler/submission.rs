use std::sync::atomic::Ordering;
use tokio::sync::{MutexGuard, mpsc};

use super::SessionHandler;
use crate::downloader::task::{DownloadTask, TaskId};
use crate::downloader::worker::QueuedDownloadTask;
use crate::error::{Error, Result};

impl SessionHandler {
    pub(crate) fn allocate_task_id(&self) -> TaskId {
        let sequence = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        format!("{}-{}", self.session_id, sequence)
    }

    pub(crate) async fn submission_guard(&self) -> MutexGuard<'_, ()> {
        self.submission.lock().await
    }

    pub(crate) async fn enqueue_task(
        &self,
        task: DownloadTask,
        group: Option<String>,
    ) -> Result<()> {
        let task_id = self.allocate_task_id();
        let sender = self.clone_sender().ok_or(Error::SessionSenderClosed)?;

        self.start_task();
        self.mark_enqueued(task_id.clone(), &task, group.clone());

        let queued = QueuedDownloadTask {
            task_id: task_id.clone(),
            task,
            group,
        };

        if let Err(error) = sender.send(queued).await {
            self.mark_cancelled(&task_id);
            self.finish_task();
            return Err(Error::EnqueueTask {
                task_id,
                source: Box::new(error),
            });
        }

        Ok(())
    }

    fn clone_sender(&self) -> Option<mpsc::Sender<QueuedDownloadTask>> {
        self.lock_sender().clone()
    }
}
