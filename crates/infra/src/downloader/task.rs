use std::{
    fmt,
    path::{Component, Path, PathBuf},
};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub use crate::error::TaskId;
use crate::error::{Error, Result};

pub type SessionId = u64;

pub(crate) struct HeaderNames<'a>(pub(crate) &'a HeaderMap);

impl fmt::Debug for HeaderNames<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.keys().map(HeaderName::as_str))
            .finish()
    }
}

pub(crate) fn insert_header(headers: &mut HeaderMap, name: &str, value: &str) -> Result<()> {
    let header_name =
        HeaderName::from_bytes(name.as_bytes()).map_err(|error| Error::InvalidHeaderName {
            name: name.to_owned(),
            reason: error.to_string(),
        })?;
    let header_value = HeaderValue::from_str(value).map_err(|error| Error::InvalidHeaderValue {
        name: name.to_owned(),
        reason: error.to_string(),
    })?;
    headers.append(header_name, header_value);
    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
pub struct DownloadTask {
    pub(crate) url: String,
    pub(crate) path: PathBuf,
    pub(crate) expected_size: Option<u64>,
    pub(crate) sha1: Option<String>,
    pub(crate) headers: HeaderMap,
}

impl fmt::Debug for DownloadTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DownloadTask")
            .field("url", &self.url)
            .field("path", &self.path)
            .field("expected_size", &self.expected_size)
            .field("sha1", &self.sha1)
            .field("headers", &HeaderNames(&self.headers))
            .finish()
    }
}

impl DownloadTask {
    pub fn new(url: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            expected_size: None,
            sha1: None,
            headers: HeaderMap::new(),
        }
    }

    pub fn size(mut self, expected_size: u64) -> Self {
        self.expected_size = Some(expected_size);
        self
    }

    pub fn sha1(mut self, sha1: impl Into<String>) -> Self {
        self.sha1 = Some(sha1.into());
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let value = value.into();
        insert_header(&mut self.headers, &name, &value)?;
        Ok(self)
    }

    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn expected_size(&self) -> Option<u64> {
        self.expected_size
    }

    pub fn sha1_checksum(&self) -> Option<&str> {
        self.sha1.as_deref()
    }

    pub fn header_map(&self) -> &HeaderMap {
        &self.headers
    }

    pub(crate) fn validate(&self) -> Result<()> {
        let url = reqwest::Url::parse(&self.url).map_err(|error| Error::InvalidTaskUrl {
            url: self.url.clone(),
            reason: error.to_string(),
        })?;

        if !matches!(url.scheme(), "http" | "https") {
            return Err(Error::InvalidTaskUrl {
                url: self.url.clone(),
                reason: format!("unsupported URL scheme '{}'", url.scheme()),
            });
        }

        if self.path.file_name().is_none() {
            return Err(Error::InvalidTaskPath {
                path: self.path.clone(),
                reason: "a target file name is required",
            });
        }

        if self
            .path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(Error::InvalidTaskPath {
                path: self.path.clone(),
                reason: "parent directory components are not allowed",
            });
        }

        if let Some(sha1) = &self.sha1 {
            validate_sha1(sha1)?;
        }

        Ok(())
    }
}

pub(crate) fn validate_sha1(sha1: &str) -> Result<()> {
    if sha1.len() != 40 || !sha1.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidSha1 {
            sha1: sha1.to_owned(),
        });
    }

    Ok(())
}

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

    pub(crate) fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub(crate) fn expected_bytes(&self) -> Option<u64> {
        self.tasks
            .iter()
            .map(DownloadTask::expected_size)
            .try_fold(0_u64, |total, size| {
                size.map(|size| total.saturating_add(size))
            })
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
