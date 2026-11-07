use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use elemental_schema::forge::ForgeInstallerProfile;
use serde::Serialize;
use tokio::fs::create_dir_all;

use crate::error::{Error, Result};

use super::InstallerStatePaths;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

impl InstallerStatePaths<'_> {
    pub(crate) async fn persist_install_profile(
        &self,
        install_profile: &ForgeInstallerProfile,
    ) -> Result<()> {
        write_json(
            self.install_profile_path(),
            install_profile,
            self.family.as_str(),
        )
        .await
    }

    pub(crate) async fn persist_embedded_version(
        &self,
        embedded_version: Option<&serde_json::Value>,
    ) -> Result<()> {
        let path = self.embedded_version_path();
        if let Some(version) = embedded_version {
            return write_json(path, version, self.family.as_str()).await;
        }

        if path.exists() {
            tokio::fs::remove_file(path).await?;
        }
        Ok(())
    }
}

async fn write_json<T: Serialize>(path: PathBuf, value: &T, family_name: &str) -> Result<()> {
    let parent = path.parent().ok_or_else(|| Error::MissingParentDirectory {
        context: format!("{family_name} installer state path has no parent directory"),
        path: path.clone(),
    })?;
    create_dir_all(parent).await?;
    let serialized = serde_json::to_vec_pretty(value)?;
    let temp_path = temporary_sibling_path(&path);
    tokio::fs::write(&temp_path, serialized).await?;

    if let Err(source) = tokio::fs::rename(&temp_path, &path).await {
        // Windows does not replace an existing destination with `rename`.
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
            tokio::fs::rename(&temp_path, &path).await?;
        } else {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(source.into());
        }
    }

    Ok(())
}

fn temporary_sibling_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "installer-state".to_owned());
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        counter
    ))
}
