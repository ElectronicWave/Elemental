use std::time::{Duration, Instant};

use super::SessionState;
use crate::downloader::event::DownloadEvent;
use crate::downloader::report::{DownloadFailure, DownloadProgress, TaskStatus};
use crate::downloader::task::{DownloadTask, TaskId};

const TASK_PROGRESS_EVENT_INTERVAL: Duration = Duration::from_millis(100);

impl SessionState {
    pub(crate) fn enqueue(
        &mut self,
        task_id: TaskId,
        task: &DownloadTask,
        group: Option<String>,
        now: Instant,
    ) -> SessionTransition {
        self.accepted += 1;
        let record = super::record::TaskRecord::new(task_id, task, group, now);
        let info = record.details.clone();
        self.insert_record(record);

        self.build_update(
            now,
            vec![DownloadEvent::TaskQueued {
                session_id: self.session_id,
                task: info,
            }],
        )
    }

    pub(crate) fn start(&mut self, task_id: &TaskId, now: Instant) -> SessionTransition {
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

    pub(crate) fn receive(
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
                    expected_bytes: record.details.expected_bytes,
                });
            }
        }

        self.build_update(now, events)
    }

    pub(crate) fn retrying(
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

    pub(crate) fn downloaded(
        &mut self,
        task_id: &TaskId,
        bytes: u64,
        duration: Duration,
        now: Instant,
    ) -> SessionTransition {
        self.downloaded += 1;
        self.bytes_downloaded = self.bytes_downloaded.saturating_add(bytes);

        self.finish_task(
            task_id,
            TaskStatus::Downloaded { bytes, duration },
            Some(bytes),
            now,
        )
    }

    pub(crate) fn skipped(
        &mut self,
        task_id: &TaskId,
        bytes: u64,
        now: Instant,
    ) -> SessionTransition {
        self.skipped += 1;
        self.bytes_skipped = self.bytes_skipped.saturating_add(bytes);

        self.finish_task(task_id, TaskStatus::Skipped { bytes }, None, now)
    }

    pub(crate) fn failed(&mut self, failure: DownloadFailure, now: Instant) -> SessionTransition {
        let task_id = failure.task_id.clone();
        self.failed += 1;

        self.finish_task(&task_id, TaskStatus::Failed { failure }, None, now)
    }

    pub(crate) fn cancelled(&mut self, task_id: &TaskId, now: Instant) -> SessionTransition {
        self.cancelled += 1;

        self.finish_task(task_id, TaskStatus::Cancelled, None, now)
    }

    fn finish_task(
        &mut self,
        task_id: &TaskId,
        status: TaskStatus,
        final_received_bytes: Option<u64>,
        now: Instant,
    ) -> SessionTransition {
        let in_progress = self
            .records
            .get(task_id)
            .map(|record| record.received_bytes)
            .unwrap_or_else(|| final_received_bytes.unwrap_or(0));
        self.bytes_in_progress = self.bytes_in_progress.saturating_sub(in_progress);

        let events = self
            .records
            .get_mut(task_id)
            .map(|record| {
                if let Some(received_bytes) = final_received_bytes {
                    record.received_bytes = received_bytes;
                }
                record.status = status;
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
pub(crate) struct SessionTransition {
    pub(crate) progress: DownloadProgress,
    pub(crate) events: Vec<DownloadEvent>,
}
