use std::{io::ErrorKind, path::Path};
use tokio::io::AsyncWriteExt;

use super::HardlinkCacheMaterializer;
use crate::downloader::materializer::{
    StagedDownload,
    fs::{cleanup_file, create_temp_output_file, next_temp_path, replace_file},
};
use crate::downloader::task::DownloadTask;
use crate::error::{Error, Result};

impl HardlinkCacheMaterializer {
    pub(super) async fn materialize_from_cache(
        &self,
        task: &DownloadTask,
        cache_path: &Path,
    ) -> Result<()> {
        let parent = task
            .path
            .parent()
            .ok_or_else(|| Error::MissingPathComponent {
                path: task.path.clone(),
                component: "target parent directory",
            })?;
        tokio::fs::create_dir_all(parent).await?;

        let target_file_name = task
            .path
            .file_name()
            .ok_or_else(|| Error::MissingPathComponent {
                path: task.path.clone(),
                component: "target file name",
            })?
            .to_string_lossy()
            .into_owned();

        for _ in 0..32 {
            let temp_path = next_temp_path(parent, &target_file_name);
            match tokio::fs::hard_link(cache_path, &temp_path).await {
                Ok(()) => {
                    let result = replace_file(&temp_path, &task.path).await;
                    if result.is_err() {
                        cleanup_file(&temp_path).await;
                    }
                    return result;
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                Err(error) if Self::should_fallback_to_copy(&error) => {
                    return self
                        .copy_file_into_place(cache_path, &task.path, parent, &target_file_name)
                        .await;
                }
                Err(error) => return Err(error.into()),
            }
        }

        Err(Error::TempFileCreation {
            directory: parent.to_path_buf(),
        })
    }

    fn should_fallback_to_copy(error: &std::io::Error) -> bool {
        matches!(
            error.kind(),
            ErrorKind::CrossesDevices | ErrorKind::Unsupported | ErrorKind::PermissionDenied
        )
    }

    async fn copy_file_into_place(
        &self,
        cache_path: &Path,
        target_path: &Path,
        parent: &Path,
        file_name: &str,
    ) -> Result<()> {
        let StagedDownload {
            path: temp_path,
            file,
        } = create_temp_output_file(parent, file_name.to_owned()).await?;

        let result = self
            .copy_file_contents(cache_path, file, &temp_path, target_path)
            .await;
        if result.is_err() {
            cleanup_file(&temp_path).await;
        }
        result
    }

    async fn copy_file_contents(
        &self,
        cache_path: &Path,
        file: Option<tokio::fs::File>,
        temp_path: &Path,
        target_path: &Path,
    ) -> Result<()> {
        let mut source = tokio::fs::File::open(cache_path).await?;
        let mut destination = file.ok_or(Error::StagedFileMissing)?;
        tokio::io::copy(&mut source, &mut destination).await?;
        destination.flush().await?;
        drop(destination);
        drop(source);
        replace_file(temp_path, target_path).await
    }
}
