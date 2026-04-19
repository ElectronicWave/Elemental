use anyhow::{Result, bail};
use sha1_smol::Sha1;
use std::path::Path;
use tokio::{fs::File, io::AsyncReadExt};

use super::task::DownloadTask;

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
}

#[derive(Clone)]
pub struct StreamingValidator {
    validation: DownloadValidation,
    actual_size: u64,
    sha1: Option<Sha1>,
}

impl StreamingValidator {
    pub fn from_task(task: &DownloadTask) -> Self {
        Self::new(DownloadValidation::from_task(task))
    }

    pub fn new(validation: DownloadValidation) -> Self {
        let sha1 = validation.expected_sha1.as_ref().map(|_| Sha1::new());
        Self {
            validation,
            actual_size: 0,
            sha1,
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.actual_size += data.len() as u64;
        if let Some(sha1) = &mut self.sha1 {
            sha1.update(data);
        }
    }

    pub fn finish(self, path: &Path) -> Result<()> {
        if let Some(expected_size) = self.validation.expected_size {
            if self.actual_size != expected_size {
                bail!(
                    "downloaded file size mismatch for '{}': expected {}, got {}",
                    path.display(),
                    expected_size,
                    self.actual_size
                );
            }
        }

        if let Some(expected_sha1) = self.validation.expected_sha1 {
            let actual_sha1 = self
                .sha1
                .unwrap_or_default()
                .digest()
                .to_string();
            if actual_sha1 != expected_sha1 {
                bail!(
                    "downloaded file sha1 mismatch for '{}': expected {}, got {}",
                    path.display(),
                    expected_sha1,
                    actual_sha1
                );
            }
        }

        Ok(())
    }
}

pub async fn validate_file(path: &Path, validation: &DownloadValidation) -> Result<()> {
    if !validation.is_enabled() {
        return Ok(());
    }

    let mut file = File::open(path).await?;
    let mut validator = StreamingValidator::new(validation.clone());
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

pub async fn target_matches_task(path: &Path, task: &DownloadTask) -> Result<bool> {
    let validation = DownloadValidation::from_task(task);
    if !validation.is_enabled() {
        return Ok(false);
    }

    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };

    if let Some(expected_size) = validation.expected_size {
        if metadata.len() != expected_size {
            return Ok(false);
        }
    }

    if validation.expected_sha1.is_none() {
        return Ok(true);
    }

    match validate_file(path, &validation).await {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}
