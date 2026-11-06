use std::time::{Duration, Instant};

use super::{SessionState, record::TaskRecord};
use crate::downloader::event::SessionSnapshot;
use crate::downloader::report::{DownloadProgress, DownloadReport};

const PROGRESS_EVENT_INTERVAL: Duration = Duration::from_millis(100);

impl SessionState {
    pub(crate) fn snapshot(&mut self, session_name: Option<String>) -> SessionSnapshot {
        let progress = self.progress(Instant::now());
        let tasks = self
            .record_order
            .iter()
            .filter_map(|task_id| self.records.get(task_id))
            .map(TaskRecord::snapshot)
            .collect();

        SessionSnapshot {
            session_id: self.session_id,
            session_name,
            progress,
            tasks,
        }
    }

    pub(crate) fn report(&self, session_name: Option<String>, elapsed: Duration) -> DownloadReport {
        let outcomes = self
            .record_order
            .iter()
            .filter_map(|task_id| self.records.get(task_id))
            .map(TaskRecord::outcome)
            .collect();
        let expected_bytes = self.expected_bytes_known.then_some(self.expected_bytes);

        DownloadReport {
            session_id: self.session_id,
            session_name,
            outcomes,
            downloaded_bytes: self.bytes_downloaded,
            skipped_bytes: self.bytes_skipped,
            expected_bytes,
            elapsed,
        }
    }

    pub(crate) fn should_emit_progress(&mut self, now: Instant) -> bool {
        if now.saturating_duration_since(self.last_progress_event) < PROGRESS_EVENT_INTERVAL {
            return false;
        }

        self.last_progress_event = now;
        true
    }

    pub(crate) fn progress(&mut self, now: Instant) -> DownloadProgress {
        let bytes_per_second = self.throughput.value(now);
        let bytes_per_second = u64::try_from(bytes_per_second).unwrap_or(u64::MAX);
        let tasks_completed = self.downloaded.saturating_add(self.skipped);
        let tasks_pending = self.accepted.saturating_sub(
            tasks_completed
                .saturating_add(self.failed)
                .saturating_add(self.cancelled),
        );
        let expected_bytes = self.expected_bytes_known.then_some(self.expected_bytes);
        let completed_bytes = self
            .bytes_downloaded
            .saturating_add(self.bytes_skipped)
            .saturating_add(self.bytes_in_progress);
        let eta = expected_bytes.and_then(|total| {
            let remaining = total.saturating_sub(completed_bytes);
            if remaining == 0 {
                Some(Duration::ZERO)
            } else {
                remaining
                    .checked_div(bytes_per_second)
                    .map(Duration::from_secs)
            }
        });

        DownloadProgress {
            tasks_total: self.accepted,
            tasks_completed,
            tasks_failed: self.failed,
            tasks_skipped: self.skipped,
            tasks_cancelled: self.cancelled,
            tasks_pending,
            bytes_downloaded: self.bytes_downloaded,
            bytes_received: self.bytes_received,
            bytes_skipped: self.bytes_skipped,
            expected_bytes,
            bytes_per_second,
            eta,
        }
    }
}
