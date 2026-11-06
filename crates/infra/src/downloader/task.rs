use std::{
    fmt,
    path::{Component, Path, PathBuf},
};

use reqwest::header::HeaderMap;

use crate::downloader::header::{HeaderNames, insert_header};
use crate::error::{Error, Result};

pub type SessionId = u64;
pub type TaskId = String;

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
