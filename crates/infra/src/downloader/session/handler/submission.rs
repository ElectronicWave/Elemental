use std::sync::atomic::Ordering;
use tokio::sync::mpsc;

use super::{QueuedDownloadTask, SessionHandler};
use crate::downloader::state::StateCommand;
use crate::downloader::task::{DownloadBatch, DownloadTask, TaskId};
use crate::error::{Error, Result};

impl SessionHandler {
    pub(crate) async fn submit_batch(&self, batch: DownloadBatch) -> Result<()> {
        batch.validate()?;
        let _submission_guard = self.submission.lock().await;
        self.lifecycle.require_accepting_tasks(self.session_id)?;

        self.apply_state(StateCommand::AcceptBatch(&batch));
        let (group, tasks) = batch.into_parts();

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
        self.apply_state(StateCommand::Enqueue {
            task_id: task_id.clone(),
            task: &task,
            group: group.clone(),
        });

        let queued = QueuedDownloadTask {
            task_id: task_id.clone(),
            task,
            group,
        };

        if let Err(error) = sender.send(queued).await {
            self.finish_cancelled(&task_id);
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
