use std::path::Path;
use tokio::{fs::File, io::AsyncReadExt};

use super::task::DownloadTask;
use crate::error::{Error, Result};

mod streaming;

pub use streaming::StreamingValidator;

const VALIDATION_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadValidation {
    pub expected_size: Option<u64>,
    pub expected_sha1: Option<String>,
}

impl DownloadValidation {
    pub fn from_task(task: &DownloadTask) -> Self {
        Self {
            expected_size: task.expected_size,
            expected_sha1: task.sha1.clone(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.expected_size.is_some() || self.expected_sha1.is_some()
    }

    pub fn streaming(&self) -> StreamingValidator {
        StreamingValidator::new(self.clone())
    }

    pub async fn validate_file(&self, path: &Path) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        let mut file = File::open(path).await?;
        let mut validator = self.streaming();
        let mut buffer = [0u8; VALIDATION_BUFFER_SIZE];

        loop {
            let read = file.read(&mut buffer).await?;
            if read == 0 {
                break;
            }

            validator.update(&buffer[..read]);
        }

        validator.finish(path)
    }

    pub async fn matches(&self, path: &Path) -> Result<bool> {
        if !self.is_enabled() {
            return Ok(false);
        }

        let metadata = match tokio::fs::metadata(path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };

        if let Some(expected_size) = self.expected_size
            && metadata.len() != expected_size
        {
            return Ok(false);
        }

        if self.expected_sha1.is_none() {
            return Ok(true);
        }

        match self.validate_file(path).await {
            Ok(()) => Ok(true),
            Err(Error::SizeMismatch { .. } | Error::Sha1Mismatch { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    }
}
