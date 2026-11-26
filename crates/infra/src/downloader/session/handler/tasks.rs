use std::{sync::atomic::Ordering, time::Instant};

use super::SessionHandler;
use crate::downloader::{report::DownloadFailure, state::StateCommand, task::TaskId};

impl SessionHandler {
    pub(crate) fn apply_state(&self, command: StateCommand<'_>) {
        let mut state = self.lock_state();
        let update = state.apply(command, Instant::now());

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
        self.apply_state(StateCommand::Failed(failure));
        self.apply_failure_policy();
        self.finish_task();
    }

    pub(crate) fn finish_cancelled(&self, task_id: TaskId) {
        self.apply_state(StateCommand::Cancelled(task_id));
        self.finish_task();
    }
}
