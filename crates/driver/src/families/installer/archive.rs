use std::path::Path;

use crate::error::Result;
use elemental_infra::jar::JarFile;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone)]
pub struct InstallerArchive<P: AsRef<Path>> {
    jar: JarFile<P>,
}

impl<P: AsRef<Path>> InstallerArchive<P> {
    pub fn new(path: P) -> Self {
        Self {
            jar: JarFile::new(path),
        }
    }

    pub fn read_bytes(&self, name: &str) -> Result<Vec<u8>> {
        Ok(self.jar.by_name_bytes(normalize_entry_name(name))?)
    }

    pub fn read_string(&self, name: &str) -> Result<String> {
        Ok(self.jar.by_name_string(normalize_entry_name(name))?)
    }

    pub fn read_json<T>(&self, name: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        Ok(serde_json::from_str(&self.read_string(name)?)?)
    }

    pub(crate) async fn extract_entry(&self, name: &str, destination: &Path) -> Result<()>
    where
        P: Clone + Send + 'static,
    {
        let archive = self.clone();
        let name = name.to_owned();
        let destination = destination.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let bytes = archive.read_bytes(name.as_str())?;
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(destination, bytes)?;
            Ok(())
        })
        .await?
    }

    pub async fn extract_maven_artifacts(&self, dest: &Path) -> Result<()>
    where
        P: Clone + Send + 'static,
    {
        self.jar.extract_prefixed("maven", dest).await?;
        Ok(())
    }
}

fn normalize_entry_name(name: &str) -> &str {
    name.trim_start_matches('/')
}
