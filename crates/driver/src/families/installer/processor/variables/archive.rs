use elemental_infra::SafeRelativePath;

use crate::{error::Result, families::installer::installer_coordinate_path};

use super::InstallerProcessorContext;

impl InstallerProcessorContext<'_> {
    pub(super) fn materialize_data_value(&self, raw: &str) -> Result<String> {
        if let Some(notation) = raw
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            return Ok(self
                .libraries_directory
                .join(installer_coordinate_path(notation)?.as_path())
                .to_string_lossy()
                .to_string());
        }

        if let Some(value) = raw
            .strip_prefix('\'')
            .and_then(|value| value.strip_suffix('\''))
        {
            return Ok(value.to_owned());
        }

        if let Some(relative_path) = raw.strip_prefix('/') {
            return self.materialize_archive_path(relative_path);
        }

        Ok(raw.to_owned())
    }

    fn materialize_archive_path(&self, relative_path: &str) -> Result<String> {
        let relative_path = SafeRelativePath::parse(relative_path)?;
        let output_path = relative_path.join_to(&self.state_directory);
        if !output_path.exists() {
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(
                &output_path,
                self.archive.read_bytes(relative_path.as_str())?,
            )?;
        }

        Ok(output_path.to_string_lossy().to_string())
    }
}
