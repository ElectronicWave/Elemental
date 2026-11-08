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
    bytes_received: u64,
    expected_bytes: Option<u64>,
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
            bytes_received: 0,
            expected_bytes: Some(0),
            records: StdHashMap::new(),
            record_order: Vec::new(),
            throughput: ThroughputMeter::default(),
            last_progress_event: Instant::now(),
        }
    }

    fn insert_record(&mut self, record: TaskRecord) {
        let task_id = record.details.task_id.clone();
        if !self.records.contains_key(&task_id) {
            self.record_order.push(task_id.clone());
        }
        self.records.insert(task_id, record);
    }
}
