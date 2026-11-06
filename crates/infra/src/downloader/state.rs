use std::{collections::HashMap as StdHashMap, time::Instant};

use super::task::{SessionId, TaskId};

mod progress;
mod record;
mod throughput;
mod transitions;

use record::TaskRecord;
use throughput::ThroughputMeter;

pub(crate) use transitions::SessionTransition;

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
    failed: usize,
    cancelled: usize,
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
            failed: 0,
            cancelled: 0,
            records: StdHashMap::new(),
            record_order: Vec::new(),
            throughput: ThroughputMeter::default(),
            last_progress_event: Instant::now(),
        }
    }

    fn insert_record(&mut self, record: TaskRecord) {
        let task_id = record.details.task_id.clone();
        match record.details.expected_bytes {
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
}
