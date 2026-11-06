use super::task::DownloadTask;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadBatch {
    group: Option<String>,
    tasks: Vec<DownloadTask>,
}

impl DownloadBatch {
    pub fn new(tasks: Vec<DownloadTask>) -> Self {
        Self { group: None, tasks }
    }

    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub(crate) fn into_parts(self) -> (Option<String>, Vec<DownloadTask>) {
        (self.group, self.tasks)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        self.tasks.iter().try_for_each(DownloadTask::validate)
    }
}

impl From<DownloadTask> for DownloadBatch {
    fn from(task: DownloadTask) -> Self {
        Self::new(vec![task])
    }
}

impl From<Vec<DownloadTask>> for DownloadBatch {
    fn from(tasks: Vec<DownloadTask>) -> Self {
        Self::new(tasks)
    }
}
