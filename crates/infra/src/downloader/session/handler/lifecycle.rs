use std::sync::atomic::{AtomicU8, Ordering};

use super::SessionHandler;
use crate::downloader::report::DownloadReport;
use crate::downloader::task::SessionId;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum LifecycleState {
    Open = 0,
    Draining = 1,
    Closed = 2,
}

impl LifecycleState {
    fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::Open,
            1 => Self::Draining,
            _ => Self::Closed,
        }
    }
}

#[derive(Debug)]
pub(super) struct SessionLifecycle {
    state: AtomicU8,
}

impl SessionLifecycle {
    pub(super) fn new() -> Self {
        Self {
            state: AtomicU8::new(LifecycleState::Open as u8),
        }
    }

    pub(super) fn ensure_accepting_tasks(&self, session_id: SessionId) -> Result<()> {
        match self.state() {
            LifecycleState::Open => Ok(()),
            LifecycleState::Draining => Err(Error::SessionNotAcceptingTasks { session_id }),
            LifecycleState::Closed => Err(Error::SessionClosed { session_id }),
        }
    }

    pub(super) fn is_draining(&self) -> bool {
        self.state() == LifecycleState::Draining
    }

    pub(super) fn close(&self) {
        self.advance(LifecycleState::Closed);
    }

    pub(super) fn stop_scheduling(&self) {
        self.advance(LifecycleState::Draining);
    }

    fn advance(&self, next: LifecycleState) {
        self.state.fetch_max(next as u8, Ordering::AcqRel);
    }

    fn state(&self) -> LifecycleState {
        LifecycleState::from_raw(self.state.load(Ordering::Acquire))
    }
}

impl SessionHandler {
    pub(crate) async fn close_and_wait(&self) {
        self.close().await;
        self.wait_idle().await;
        self.wait_workers().await;
    }

    pub(crate) async fn finish(&self) -> DownloadReport {
        self.close_and_wait().await;
        self.emit_closed_report()
    }

    pub(crate) async fn cancel(&self) -> DownloadReport {
        self.cancel_in_flight();
        self.close_and_wait().await;
        self.emit_closed_report()
    }

    async fn close(&self) {
        let _submission_guard = self.submission.lock().await;
        self.lifecycle.close();
        self.workers.close();
        self.lock_sender().take();
    }

    pub(crate) fn abort_detached(&self) {
        self.lifecycle.stop_scheduling();
        self.token.cancel();
        self.workers.close();
        self.lock_sender().take();
    }

    pub(crate) fn is_draining(&self) -> bool {
        self.lifecycle.is_draining()
    }

    pub(crate) fn apply_failure_policy(&self) {
        let policy = self.failure_policy;
        if policy.stops_scheduling() {
            self.lifecycle.stop_scheduling();
        }
        if policy.cancels_in_flight() {
            self.token.cancel();
        }
    }

    fn cancel_in_flight(&self) {
        self.lifecycle.stop_scheduling();
        self.token.cancel();
    }
}
