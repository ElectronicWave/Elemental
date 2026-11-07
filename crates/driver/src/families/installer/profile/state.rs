use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use elemental_infra::SafeRelativePath;
use elemental_schema::forge::ForgeInstallerProfile;
use serde::Serialize;
use tokio::fs::create_dir_all;

use crate::{
    error::{Error, Result},
    families::installer::InstallerArchive,
};

const INSTALL_PROFILE_ENTRY: &str = "install_profile.json";
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct InstallerPersistedState {
    pub install_profile: ForgeInstallerProfile,
    pub embedded_version: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub(crate) struct InstallerStatePaths<'a> {
    instance_root: &'a Path,
    family: SafeRelativePath,
}

impl<'a> InstallerStatePaths<'a> {
    pub(crate) fn new(instance_root: &'a Path, family_name: &str) -> Result<Self> {
        Ok(Self {
            instance_root,
            family: SafeRelativePath::parse(family_name.to_owned())?,
        })
    }

    pub(crate) fn install_profile_path(&self) -> PathBuf {
        self.relative_path(PathBuf::from(INSTALL_PROFILE_ENTRY))
    }

    pub(crate) fn embedded_version_path(&self) -> PathBuf {
        self.relative_path(PathBuf::from("version.json"))
    }

    fn root(&self) -> PathBuf {
        self.instance_root
            .join(".elemental")
            .join(self.family.as_path())
    }

    fn relative_path(&self, relative_path: PathBuf) -> PathBuf {
        self.root().join(relative_path)
    }

    pub(crate) fn load(&self) -> Result<InstallerPersistedState> {
        let install_profile_path = self.install_profile_path();
        let install_profile = serde_json::from_reader(std::fs::File::open(&install_profile_path)?)
            .map_err(|source| Error::ReadPersistedInstallProfile {
                family_name: self.family.as_str().to_owned(),
                path: install_profile_path.clone(),
                source,
            })?;

        let embedded_version_path = self.embedded_version_path();
        let embedded_version = if embedded_version_path.exists() {
            Some(
                serde_json::from_reader(std::fs::File::open(&embedded_version_path)?).map_err(
                    |source| Error::ReadPersistedEmbeddedVersion {
                        family_name: self.family.as_str().to_owned(),
                        path: embedded_version_path.clone(),
                        source,
                    },
                )?,
            )
        } else {
            None
        };

        Ok(InstallerPersistedState {
            install_profile,
            embedded_version,
        })
    }

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

pub(crate) fn read_embedded_version<P: AsRef<Path>>(
    archive: &InstallerArchive<P>,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
) -> Result<Option<serde_json::Value>> {
    if let Some(version_info) = &install_profile.version_info {
        return Ok(Some(version_info.clone()));
    }

    let Some(json_path) = install_profile.json.as_deref() else {
        return Ok(None);
    };

    Ok(Some(
        archive
            .read_json::<serde_json::Value>(json_path)
            .map_err(|source| Error::ReadEmbeddedVersionJson {
                family_name: family_name.to_owned(),
                path: json_path.to_owned(),
                source: Box::new(source),
            })?,
    ))
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
