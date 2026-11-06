use std::sync::atomic::{AtomicU8, Ordering};

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

    pub(super) fn is_closed(&self) -> bool {
        self.state() != LifecycleState::Open
    }

    pub(super) fn is_draining(&self) -> bool {
        self.state() == LifecycleState::Draining
    }

    pub(super) fn close(&self) {
        let _ = self.state.compare_exchange(
            LifecycleState::Open as u8,
            LifecycleState::Closed as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    pub(super) fn stop_scheduling(&self) {
        self.state
            .store(LifecycleState::Draining as u8, Ordering::Release);
    }

    fn state(&self) -> LifecycleState {
        LifecycleState::from_raw(self.state.load(Ordering::Acquire))
    }
}
