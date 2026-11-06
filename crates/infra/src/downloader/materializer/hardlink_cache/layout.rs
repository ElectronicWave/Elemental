use std::path::PathBuf;

use super::HardlinkCacheMaterializer;
use crate::downloader::task::{DownloadTask, validate_sha1};
use crate::error::{Error, Result};

impl HardlinkCacheMaterializer {
    pub(super) fn cache_path_for_task(&self, task: &DownloadTask) -> Result<PathBuf> {
        let raw_sha1 = task.sha1.as_ref().ok_or(Error::HardlinkCacheSha1Missing)?;
        validate_sha1(raw_sha1)?;
        let sha1 = raw_sha1.to_ascii_lowercase();
        let prefix = sha1
            .get(0..2)
            .ok_or_else(|| Error::Sha1TooShort { sha1: sha1.clone() })?;
        Ok(self.cache_root.join(prefix).join(sha1))
    }
}
