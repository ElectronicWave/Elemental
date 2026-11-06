use std::sync::atomic::Ordering;
use tokio::sync::mpsc;

use super::SessionHandler;
use crate::downloader::batch::DownloadBatch;
use crate::downloader::task::{DownloadTask, TaskId};
use crate::downloader::worker::QueuedDownloadTask;
use crate::error::{Error, Result};

impl SessionHandler {
    pub(crate) async fn submit_batch(&self, batch: DownloadBatch) -> Result<()> {
        batch.validate()?;
        let (group, tasks) = batch.into_parts();
        let _submission_guard = self.submission.lock().await;
        self.lifecycle.ensure_accepting_tasks(self.session_id)?;

        for task in tasks {
            self.enqueue_task(task, group.clone()).await?;
        }

        Ok(())
    }

    pub(crate) fn allocate_task_id(&self) -> TaskId {
        let sequence = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        format!("{}-{}", self.session_id, sequence)
    }

    async fn enqueue_task(&self, task: DownloadTask, group: Option<String>) -> Result<()> {
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
