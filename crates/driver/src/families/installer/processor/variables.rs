use std::path::PathBuf;

use tokio::fs::create_dir_all;

use crate::error::{Error, Result};

use super::context::{CLIENT_PROCESSOR_DATA_KEYS, InstallerProcessorContext};

mod archive;

impl InstallerProcessorContext<'_> {
    pub(super) async fn ensure_client_directories(&self) -> Result<()> {
        for path in self.required_client_output_paths().await? {
            if let Some(parent) = path.parent() {
                create_dir_all(parent).await?;
            }
        }

        Ok(())
    }

    pub(super) async fn client_outputs_ready(&self) -> Result<bool> {
        let paths = self.required_client_output_paths().await?;
        if paths.is_empty() {
            return Ok(false);
        }

        Ok(paths.iter().all(|path| path.exists()))
    }

    async fn required_client_output_paths(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for key in CLIENT_PROCESSOR_DATA_KEYS {
            if let Some(value) = self.resolve_data_value(key).await? {
                paths.push(PathBuf::from(value));
            }
        }

        Ok(paths)
    }

    pub(super) async fn resolve_argument(&self, value: &str) -> Result<String> {
        let mut resolved = String::with_capacity(value.len());
        let mut rest = value;

        while let Some(start) = rest.find('{') {
            resolved.push_str(&rest[..start]);
            let after_open = &rest[start + 1..];
            let Some(end) = after_open.find('}') else {
                resolved.push_str(&rest[start..]);
                rest = "";
                break;
            };
            let key = &after_open[..end];
            if key.is_empty() {
                resolved.push('{');
                rest = after_open;
                continue;
            }
            let replacement = self.resolve_variable(key).await?;
            resolved.push_str(replacement.as_str());
            rest = &after_open[end + 1..];
        }

        resolved.push_str(rest);
        self.materialize_data_value(resolved.as_str()).await
    }

    async fn resolve_variable(&self, key: &str) -> Result<String> {
        match key {
            "INSTALLER" => absolute_display(&self.installer_artifact.path),
            "ROOT" => Ok(self.root_directory.to_string_lossy().to_string()),
            "MINECRAFT_JAR" => Ok(self.minecraft_jar_path.to_string_lossy().to_string()),
            "SIDE" => Ok("client".to_owned()),
            _ => {
                self.resolve_data_value(key)
                    .await?
                    .ok_or_else(|| Error::UnknownProcessorVariable {
                        family_name: self.family_name.to_owned(),
                        key: key.to_owned(),
                    })
            }
        }
    }

    pub(super) async fn resolve_data_value(&self, key: &str) -> Result<Option<String>> {
        let Some(entry) = self.install_profile.data.get(key) else {
            return Ok(None);
        };
        let Some(raw) = entry.client.as_deref().or(entry.server.as_deref()) else {
            return Ok(None);
        };
        self.materialize_data_value(raw).await.map(Some)
    }
}

fn absolute_display(path: &std::path::Path) -> Result<String> {
    if path.is_absolute() {
        return Ok(path.to_string_lossy().to_string());
    }

    Ok(std::env::current_dir()?
        .join(path)
        .to_string_lossy()
        .to_string())
}
