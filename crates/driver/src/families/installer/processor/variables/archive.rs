use elemental_infra::SafeRelativePath;

use crate::{error::Result, families::installer::installer_coordinate_path};

use super::InstallerProcessorContext;

impl InstallerProcessorContext<'_> {
    pub(super) async fn materialize_data_value(&self, raw: &str) -> Result<String> {
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
            return self.materialize_archive_path(relative_path).await;
        }

        Ok(raw.to_owned())
    }

    async fn materialize_archive_path(&self, relative_path: &str) -> Result<String> {
        let relative_path = SafeRelativePath::parse(relative_path)?;
        let output_path = relative_path.join_to(&self.state_directory);
        if !tokio::fs::try_exists(&output_path).await? {
            self.archive
                .extract_entry(relative_path.as_str(), &output_path)
                .await?;
        }

        Ok(output_path.to_string_lossy().to_string())
    }
}
