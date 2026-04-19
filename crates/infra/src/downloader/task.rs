use std::path::PathBuf;

pub type SessionId = u64;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DownloadTask {
    pub url: String,
    pub path: PathBuf,
    pub expected_size: Option<u64>,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadPlan {
    pub session_name: Option<String>,
    pub tasks: Vec<DownloadTask>,
}

impl DownloadPlan {
    pub fn new(tasks: Vec<DownloadTask>) -> Self {
        Self {
            session_name: None,
            tasks,
        }
    }

    pub fn named(name: impl Into<String>, tasks: Vec<DownloadTask>) -> Self {
        Self {
            session_name: Some(name.into()),
            tasks,
        }
    }
}

impl DownloadTask {
    pub fn new(
        url: impl Into<String>,
        path: impl Into<PathBuf>,
        expected_size: Option<u64>,
        sha1: Option<String>,
    ) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            expected_size,
            sha1,
        }
    }
}
