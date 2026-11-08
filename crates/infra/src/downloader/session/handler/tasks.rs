use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

use super::SessionHandler;
use crate::downloader::{
    batch::DownloadBatch,
    report::DownloadFailure,
    state::{SessionState, SessionTransition},
    task::{DownloadTask, TaskId},
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
        self.with_state(|state| state.accept_batch(batch, Instant::now()));
    }

    pub(crate) fn mark_enqueued(
        &self,
        task_id: TaskId,
        task: &DownloadTask,
        group: Option<String>,
    ) {
        self.with_state(|state| state.enqueue(task_id, task, group, Instant::now()));
    }

    pub(crate) fn mark_downloaded(&self, task_id: &TaskId, bytes: u64, duration: Duration) {
        self.with_state(|state| state.downloaded(task_id, bytes, duration, Instant::now()));
    }

    pub(crate) fn mark_skipped(&self, task_id: &TaskId, bytes: u64) {
        self.with_state(|state| state.skipped(task_id, bytes, Instant::now()));
    }

    pub(crate) fn mark_active(&self, task_id: &TaskId) {
        self.with_state(|state| state.start(task_id, Instant::now()));
    }

    pub(crate) fn mark_failed(&self, failure: DownloadFailure) {
        self.with_state(|state| state.failed(failure, Instant::now()));
    }

    pub(crate) fn mark_cancelled(&self, task_id: &TaskId) {
        self.with_state(|state| state.cancelled(task_id, Instant::now()));
    }

    pub(crate) fn record_received(&self, task_id: &TaskId, bytes: usize) {
        self.with_state(|state| state.receive(task_id, bytes, Instant::now()));
    }

    pub(crate) fn mark_retrying(
        &self,
        task_id: &TaskId,
        failed_attempt: usize,
        delay: Duration,
        reason: String,
    ) {
        self.with_state(|state| {
            state.retrying(task_id, failed_attempt, delay, reason, Instant::now())
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
