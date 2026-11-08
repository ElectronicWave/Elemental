use std::time::{Duration, Instant};

use super::{SessionState, record::TaskRecord};
use crate::downloader::event::SessionSnapshot;
use crate::downloader::report::{DownloadProgress, DownloadReport, TaskStatus};

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
        let mut downloaded_bytes = 0_u64;
        let mut skipped_bytes = 0_u64;
        let mut outcomes = Vec::with_capacity(self.record_order.len());
        for task_id in &self.record_order {
            let Some(record) = self.records.get(task_id) else {
                continue;
            };

            match &record.status {
                TaskStatus::Downloaded { bytes, .. } => {
                    downloaded_bytes = downloaded_bytes.saturating_add(*bytes);
                }
                TaskStatus::Skipped { bytes } => {
                    skipped_bytes = skipped_bytes.saturating_add(*bytes);
                }
                _ => {}
            }
            outcomes.push(record.outcome());
        }

        DownloadReport {
            session_id: self.session_id,
            session_name,
            outcomes,
            downloaded_bytes,
            skipped_bytes,
            expected_bytes: self.expected_bytes,
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
        let mut tasks_completed = 0_usize;
        let mut tasks_failed = 0_usize;
        let mut tasks_skipped = 0_usize;
        let mut tasks_cancelled = 0_usize;
        let mut bytes_downloaded = 0_u64;
        let mut bytes_skipped = 0_u64;
        let mut bytes_in_progress = 0_u64;
        let mut active_tasks = Vec::new();
        let mut last_group = None;

        for task_id in &self.record_order {
            let Some(record) = self.records.get(task_id) else {
                continue;
            };

            if let Some(group) = &record.details.group {
                last_group = Some(group);
            }
            match &record.status {
                TaskStatus::Active => {
                    bytes_in_progress = bytes_in_progress.saturating_add(record.received_bytes);
                    active_tasks.push(record.active_task());
                }
                TaskStatus::Downloaded { bytes, .. } => {
                    tasks_completed = tasks_completed.saturating_add(1);
                    bytes_downloaded = bytes_downloaded.saturating_add(*bytes);
                }
                TaskStatus::Skipped { bytes } => {
                    tasks_completed = tasks_completed.saturating_add(1);
                    tasks_skipped = tasks_skipped.saturating_add(1);
                    bytes_skipped = bytes_skipped.saturating_add(*bytes);
                }
                TaskStatus::Failed { .. } => {
                    tasks_failed = tasks_failed.saturating_add(1);
                }
                TaskStatus::Cancelled => {
                    tasks_cancelled = tasks_cancelled.saturating_add(1);
                }
                TaskStatus::Queued => {}
            }
        }

        let tasks_pending = self.accepted.saturating_sub(
            tasks_completed
                .saturating_add(tasks_failed)
                .saturating_add(tasks_cancelled),
        );
        let completed_bytes = bytes_downloaded
            .saturating_add(bytes_skipped)
            .saturating_add(bytes_in_progress);
        let eta = self.expected_bytes.and_then(|total| {
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
            tasks_accepted: self.accepted,
            tasks_completed,
            tasks_failed,
            tasks_skipped,
            tasks_cancelled,
            tasks_pending,
            bytes_downloaded,
            bytes_received: self.bytes_received,
            bytes_skipped,
            expected_bytes: self.expected_bytes,
            bytes_per_second,
            eta,
            last_group: last_group.cloned(),
            active_tasks,
        }
    }
}
