use std::sync::LazyLock;

use regex::Regex;

use crate::error::{Error, Result};

use super::context::InstallerProcessorContext;

mod archive;

static VARIABLE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{([^}]+)\}").expect("processor variable regex is valid"));

impl InstallerProcessorContext<'_> {
    pub(super) fn resolve_argument(&self, value: &str) -> Result<String> {
        let mut resolved = value.to_owned();

        for capture in VARIABLE_PATTERN.captures_iter(value) {
            let Some(key) = capture.get(1).map(|group| group.as_str()) else {
                continue;
            };
            let replacement = self.resolve_variable(key)?;
            resolved = resolved.replace(&format!("{{{key}}}"), replacement.as_str());
        }

        self.materialize_data_value(resolved.as_str())
    }

    fn resolve_variable(&self, key: &str) -> Result<String> {
        match key {
            "INSTALLER" => absolute_display(&self.installer_artifact.path),
            "ROOT" => Ok(self.root_directory.to_string_lossy().to_string()),
            "MINECRAFT_JAR" => Ok(self.minecraft_jar_path.to_string_lossy().to_string()),
            "SIDE" => Ok("client".to_owned()),
            _ => self
                .resolve_data_value(key)?
                .ok_or_else(|| Error::UnknownProcessorVariable {
                    family_name: self.family_name.to_owned(),
                    key: key.to_owned(),
                }),
        }
    }

    pub(super) fn resolve_data_value(&self, key: &str) -> Result<Option<String>> {
        let Some(entry) = self.install_profile.data.get(key) else {
            return Ok(None);
        };
        let Some(raw) = entry.client.as_deref().or(entry.server.as_deref()) else {
            return Ok(None);
        };
        self.materialize_data_value(raw).map(Some)
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
