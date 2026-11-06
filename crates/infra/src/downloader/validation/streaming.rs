use std::path::Path;

use sha1::{Digest, Sha1};

use super::DownloadValidation;
use crate::downloader::task::validate_sha1;
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct StreamingValidator {
    validation: DownloadValidation,
    actual_size: u64,
    sha1: Option<Sha1>,
}

impl StreamingValidator {
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
        if let Some(expected_size) = self.validation.expected_size
            && self.actual_size != expected_size
        {
            return Err(Error::SizeMismatch {
                path: path.to_path_buf(),
                expected: expected_size,
                actual: self.actual_size,
            });
        }

        if let Some(expected_sha1) = self.validation.expected_sha1 {
            validate_sha1(&expected_sha1)?;
            let actual_sha1 = hex::encode(self.sha1.unwrap_or_default().finalize());
            if actual_sha1 != expected_sha1.to_ascii_lowercase() {
                return Err(Error::Sha1Mismatch {
                    path: path.to_path_buf(),
                    expected: expected_sha1,
                    actual: actual_sha1,
                });
            }
        }

        Ok(())
    }
}
