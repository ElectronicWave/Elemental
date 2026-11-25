use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

use super::SessionHandler;
use crate::downloader::{
    report::DownloadFailure,
    state::{SessionState, SessionTransition, StateCommand},
    task::{DownloadBatch, DownloadTask, TaskId},
};

impl SessionHandler {
    fn with_state(&self, transition: impl FnOnce(&mut SessionState) -> SessionTransition) {
        let mut state = self.lock_state();
        let update = transition(&mut state);

        if let Some(progress) = update.progress {
            self.progress.send_replace(progress);
        }
        for event in update.events {
            self.emit(event);
        }
    }

    pub(crate) fn start_task(&self) {
        self.pending_tasks.fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn mark_batch_accepted(&self, batch: &DownloadBatch) {
        self.with_state(|state| state.apply(StateCommand::AcceptBatch(batch), Instant::now()));
    }

    pub(crate) fn mark_enqueued(
        &self,
        task_id: TaskId,
        task: &DownloadTask,
        group: Option<String>,
    ) {
        self.with_state(|state| {
            state.apply(
                StateCommand::Enqueue {
                    task_id,
                    task,
                    group,
                },
                Instant::now(),
            )
        });
    }

    pub(crate) fn mark_downloaded(&self, task_id: &TaskId, bytes: u64, duration: Duration) {
        self.with_state(|state| {
            state.apply(
                StateCommand::Downloaded {
                    task_id,
                    bytes,
                    duration,
                },
                Instant::now(),
            )
        });
    }

    pub(crate) fn mark_skipped(&self, task_id: &TaskId, bytes: u64) {
        self.with_state(|state| {
            state.apply(StateCommand::Skipped { task_id, bytes }, Instant::now())
        });
    }

    pub(crate) fn mark_active(&self, task_id: &TaskId) {
        self.with_state(|state| state.apply(StateCommand::Start(task_id), Instant::now()));
    }

    pub(crate) fn mark_failed(&self, failure: DownloadFailure) {
        self.with_state(|state| state.apply(StateCommand::Failed(failure), Instant::now()));
    }

    pub(crate) fn mark_cancelled(&self, task_id: &TaskId) {
        self.with_state(|state| state.apply(StateCommand::Cancelled(task_id), Instant::now()));
    }

    pub(crate) fn record_received(&self, task_id: &TaskId, bytes: usize) {
        self.with_state(|state| {
            state.apply(StateCommand::Receive { task_id, bytes }, Instant::now())
        });
    }

    pub(crate) fn mark_retrying(
        &self,
        task_id: &TaskId,
        failed_attempt: usize,
        delay: Duration,
        reason: String,
    ) {
        self.with_state(|state| {
            state.apply(
                StateCommand::Retrying {
                    task_id,
                    failed_attempt,
                    delay,
                    reason,
                },
                Instant::now(),
            )
        });
    }

    pub(crate) fn finish_task(&self) {
        if self.pending_tasks.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.refresh_progress();
            self.idle.notify_waiters();
        }
    }

    fn refresh_progress(&self) {
        let progress = self.lock_state().progress(Instant::now());
        self.progress.send_replace(progress);
    }

    pub(crate) fn finish_failed(&self, failure: DownloadFailure) {
        self.mark_failed(failure);
        self.apply_failure_policy();
        self.finish_task();
    }

    pub(crate) fn finish_cancelled(&self, task_id: &TaskId) {
        self.mark_cancelled(task_id);
        self.finish_task();
    }
}
