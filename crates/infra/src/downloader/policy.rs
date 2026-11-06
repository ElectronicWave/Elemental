use std::time::Duration;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: usize,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl RetryPolicy {
    pub fn none() -> Self {
        Self {
            max_attempts: 1,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.max_attempts == 0 || self.max_backoff < self.initial_backoff {
            return Err(Error::InvalidRetryPolicy);
        }

        Ok(())
    }

    pub(crate) fn backoff(&self, retry_index: usize) -> Duration {
        let exponent = u32::try_from(retry_index.saturating_sub(1))
            .unwrap_or(u32::MAX)
            .min(31);
        self.initial_backoff
            .saturating_mul(2u32.saturating_pow(exponent))
            .min(self.max_backoff)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(8),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FailurePolicy {
    /// Keep downloading queued tasks and accept new submissions.
    #[default]
    Continue,
    /// Let in-flight tasks finish, cancel queued tasks, and reject new submissions.
    FinishInFlight,
    /// Cancel in-flight and queued tasks immediately and reject new submissions.
    CancelAll,
}

impl FailurePolicy {
    pub(crate) fn stops_scheduling(self) -> bool {
        !matches!(self, Self::Continue)
    }

    pub(crate) fn cancels_in_flight(self) -> bool {
        matches!(self, Self::CancelAll)
    }
}
