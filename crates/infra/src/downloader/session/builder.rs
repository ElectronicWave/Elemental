use std::sync::Arc;

use reqwest::header::HeaderMap;

use super::{DownloadSession, DownloadSessionOptions, SessionHandler, SessionRuntime};
use crate::downloader::config::{DownloadRateLimit, FailurePolicy, RetryPolicy};
use crate::downloader::event::DownloadEvent;
use crate::downloader::task::{SessionId, insert_header};
use crate::error::Result;

#[derive(Debug)]
pub struct DownloadSessionBuilder {
    runtime: Arc<SessionRuntime>,
    name: String,
    options: DownloadSessionOptions,
}

impl DownloadSessionBuilder {
    pub(in crate::downloader) fn new(runtime: Arc<SessionRuntime>, name: String) -> Self {
        Self {
            runtime,
            name,
            options: DownloadSessionOptions::default(),
        }
    }

    pub fn parallelism(mut self, parallelism: usize) -> Self {
        self.options.parallelism = Some(parallelism);
        self
    }

    pub fn retry(mut self, retry_policy: RetryPolicy) -> Self {
        self.options.retry_policy = retry_policy;
        self
    }

    pub fn failure_policy(mut self, failure_policy: FailurePolicy) -> Self {
        self.options.failure_policy = failure_policy;
        self
    }

    pub fn rate_limit(mut self, rate_limit: DownloadRateLimit) -> Self {
        self.options.rate_limit = Some(rate_limit);
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let value = value.into();
        insert_header(&mut self.options.headers, &name, &value)?;
        Ok(self)
    }

    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.options.headers = headers;
        self
    }

    pub async fn open(self) -> Result<DownloadSession> {
        self.options.validate()?;
        let session_id = SessionId::new();
        let name = Some(self.name);
        let parallelism = self
            .options
            .effective_parallelism(self.runtime.default_session_parallelism);
        let (handler, receiver, progress) = SessionHandler::create(
            session_id,
            name.clone(),
            self.options,
            self.runtime.session_queue_capacity,
            self.runtime.events.clone(),
        )?;
        handler.spawn_workers(&self.runtime, receiver, parallelism);
        handler.emit(DownloadEvent::SessionOpened {
            session_id,
            session_name: name.clone(),
        });

        Ok(DownloadSession::new(session_id, name, handler, progress))
    }
}
