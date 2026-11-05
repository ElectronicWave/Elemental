use std::{
    collections::HashMap as StdHashMap,
    time::{Duration, Instant},
};

use super::event::{DownloadEvent, SessionSnapshot};
use super::report::{DownloadFailure, DownloadProgress, DownloadReport, TaskStatus};
use super::task::{DownloadTask, SessionId, TaskId};

mod record;
mod throughput;

use record::TaskRecord;
use throughput::ThroughputMeter;

const PROGRESS_EVENT_INTERVAL: Duration = Duration::from_millis(100);
const TASK_PROGRESS_EVENT_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub(super) struct SessionState {
    session_id: SessionId,
    accepted: usize,
    downloaded: usize,
    skipped: usize,
    bytes_downloaded: u64,
    bytes_skipped: u64,
    bytes_received: u64,
    bytes_in_progress: u64,
    expected_bytes: u64,
    expected_bytes_known: bool,
    failures: Vec<DownloadFailure>,
    cancelled_task_ids: Vec<TaskId>,
    records: StdHashMap<TaskId, TaskRecord>,
    record_order: Vec<TaskId>,
    throughput: ThroughputMeter,
    last_progress_event: Instant,
}

impl SessionState {
    pub(super) fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            accepted: 0,
            downloaded: 0,
            skipped: 0,
            bytes_downloaded: 0,
            bytes_skipped: 0,
            bytes_received: 0,
            bytes_in_progress: 0,
            expected_bytes: 0,
            expected_bytes_known: true,
            failures: Vec::new(),
            cancelled_task_ids: Vec::new(),
            records: StdHashMap::new(),
            record_order: Vec::new(),
            throughput: ThroughputMeter::default(),
            last_progress_event: Instant::now(),
        }
    }

    pub(super) fn snapshot(&mut self, session_name: Option<String>) -> SessionSnapshot {
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

    pub(super) fn report(&self, session_name: Option<String>, elapsed: Duration) -> DownloadReport {
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

    fn insert_record(&mut self, record: TaskRecord) {
        let task_id = record.task_id.clone();
        match record.expected_bytes {
            Some(size) => {
                self.expected_bytes = self.expected_bytes.saturating_add(size);
            }
            None => {
                self.expected_bytes_known = false;
            }
        }
        if !self.records.contains_key(&task_id) {
            self.record_order.push(task_id.clone());
        }
        self.records.insert(task_id, record);
    }

    fn should_emit_progress(&mut self, now: Instant) -> bool {
        if now.saturating_duration_since(self.last_progress_event) < PROGRESS_EVENT_INTERVAL {
            return false;
        }

        self.last_progress_event = now;
        true
    }

    fn progress(&mut self, now: Instant) -> DownloadProgress {
        let bytes_per_second = self.throughput.value(now);
        let bytes_per_second = u64::try_from(bytes_per_second).unwrap_or(u64::MAX);
        let tasks_completed = self.downloaded.saturating_add(self.skipped);
        let tasks_pending = self.accepted.saturating_sub(
            tasks_completed
                .saturating_add(self.failures.len())
                .saturating_add(self.cancelled_task_ids.len()),
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
            tasks_failed: self.failures.len(),
            tasks_skipped: self.skipped,
            tasks_cancelled: self.cancelled_task_ids.len(),
            tasks_pending,
            bytes_downloaded: self.bytes_downloaded,
            bytes_received: self.bytes_received,
            bytes_skipped: self.bytes_skipped,
            expected_bytes,
            bytes_per_second,
            eta,
        }
    }

    pub(super) fn enqueue(
        &mut self,
        task_id: TaskId,
        task: &DownloadTask,
        group: Option<String>,
        now: Instant,
    ) -> SessionTransition {
        self.accepted += 1;
        let record = TaskRecord::new(task_id, task, group, now);
        let info = record.info();
        self.insert_record(record);

        self.build_update(
            now,
            vec![DownloadEvent::TaskQueued {
                session_id: self.session_id,
                task: info,
            }],
        )
    }

    pub(super) fn start(&mut self, task_id: &TaskId, now: Instant) -> SessionTransition {
        if let Some(record) = self.records.get_mut(task_id) {
            record.status = TaskStatus::Active;
        }

        self.build_update(
            now,
            vec![DownloadEvent::TaskStarted {
                session_id: self.session_id,
                task_id: task_id.clone(),
            }],
        )
    }

    pub(super) fn receive(
        &mut self,
        task_id: &TaskId,
        bytes: usize,
        now: Instant,
    ) -> SessionTransition {
        let received_bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
        self.bytes_received = self.bytes_received.saturating_add(received_bytes);
        self.bytes_in_progress = self.bytes_in_progress.saturating_add(received_bytes);
        self.throughput.record(bytes, now);

        let mut events = Vec::new();
        if let Some(record) = self.records.get_mut(task_id) {
            record.received_bytes = record.received_bytes.saturating_add(received_bytes);
            if now.saturating_duration_since(record.last_progress_event)
                >= TASK_PROGRESS_EVENT_INTERVAL
            {
                record.last_progress_event = now;
                events.push(DownloadEvent::TaskProgress {
                    session_id: self.session_id,
                    task_id: task_id.clone(),
                    received_bytes: record.received_bytes,
                    expected_bytes: record.expected_bytes,
                });
            }
        }

        self.build_update(now, events)
    }

    pub(super) fn retrying(
        &mut self,
        task_id: &TaskId,
        failed_attempt: usize,
        delay: Duration,
        reason: String,
        now: Instant,
    ) -> SessionTransition {
        if let Some(record) = self.records.get_mut(task_id) {
            self.bytes_in_progress = self.bytes_in_progress.saturating_sub(record.received_bytes);
            record.received_bytes = 0;
            record.last_progress_event = now;
        }

        self.build_update(
            now,
            vec![DownloadEvent::TaskRetrying {
                session_id: self.session_id,
                task_id: task_id.clone(),
                failed_attempt,
                delay_ms: u64::try_from(delay.as_millis()).unwrap_or(u64::MAX),
                reason,
            }],
        )
    }

    pub(super) fn downloaded(
        &mut self,
        task_id: &TaskId,
        bytes: u64,
        duration: Duration,
        now: Instant,
    ) -> SessionTransition {
        let received_bytes = self
            .records
            .get(task_id)
            .map(|record| record.received_bytes)
            .unwrap_or(bytes);
        self.bytes_in_progress = self.bytes_in_progress.saturating_sub(received_bytes);
        self.downloaded += 1;
        self.bytes_downloaded = self.bytes_downloaded.saturating_add(bytes);

        let events = self
            .records
            .get_mut(task_id)
            .map(|record| {
                record.received_bytes = bytes;
                record.status = TaskStatus::Downloaded { bytes, duration };
                DownloadEvent::TaskFinished {
                    session_id: self.session_id,
                    outcome: record.outcome(),
                }
            })
            .into_iter()
            .collect();

        self.build_update(now, events)
    }

    pub(super) fn skipped(
        &mut self,
        task_id: &TaskId,
        bytes: u64,
        now: Instant,
    ) -> SessionTransition {
        let received_bytes = self
            .records
            .get(task_id)
            .map(|record| record.received_bytes)
            .unwrap_or(0);
        self.bytes_in_progress = self.bytes_in_progress.saturating_sub(received_bytes);
        self.skipped += 1;
        self.bytes_skipped = self.bytes_skipped.saturating_add(bytes);

        let events = self
            .records
            .get_mut(task_id)
            .map(|record| {
                record.status = TaskStatus::Skipped { bytes };
                DownloadEvent::TaskFinished {
                    session_id: self.session_id,
                    outcome: record.outcome(),
                }
            })
            .into_iter()
            .collect();

        self.build_update(now, events)
    }

    pub(super) fn failed(&mut self, failure: DownloadFailure, now: Instant) -> SessionTransition {
        let task_id = failure.task_id.clone();
        let received_bytes = self
            .records
            .get(&task_id)
            .map(|record| record.received_bytes)
            .unwrap_or(0);
        self.bytes_in_progress = self.bytes_in_progress.saturating_sub(received_bytes);
        self.failures.push(failure.clone());

        let events = self
            .records
            .get_mut(&task_id)
            .map(|record| {
                record.status = TaskStatus::Failed { failure };
                DownloadEvent::TaskFinished {
                    session_id: self.session_id,
                    outcome: record.outcome(),
                }
            })
            .into_iter()
            .collect();

        self.build_update(now, events)
    }

    pub(super) fn cancelled(&mut self, task_id: &TaskId, now: Instant) -> SessionTransition {
        let received_bytes = self
            .records
            .get(task_id)
            .map(|record| record.received_bytes)
            .unwrap_or(0);
        self.bytes_in_progress = self.bytes_in_progress.saturating_sub(received_bytes);
        self.cancelled_task_ids.push(task_id.clone());

        let events = self
            .records
            .get_mut(task_id)
            .map(|record| {
                record.status = TaskStatus::Cancelled;
                DownloadEvent::TaskFinished {
                    session_id: self.session_id,
                    outcome: record.outcome(),
                }
            })
            .into_iter()
            .collect();

        self.build_update(now, events)
    }

    fn build_update(&mut self, now: Instant, mut events: Vec<DownloadEvent>) -> SessionTransition {
        let progress = self.progress(now);
        if self.should_emit_progress(now) {
            events.push(DownloadEvent::SessionProgress {
                session_id: self.session_id,
                progress: progress.clone(),
            });
        }

        SessionTransition { progress, events }
    }
}

#[derive(Debug)]
pub(super) struct SessionTransition {
    pub(super) progress: DownloadProgress,
    pub(super) events: Vec<DownloadEvent>,
}
