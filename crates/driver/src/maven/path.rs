use elemental_infra::SafeRelativePath;

use crate::error::{Error, Result};

use super::MavenNotation;

impl MavenNotation {
    pub fn version_directory_path(&self) -> Result<SafeRelativePath> {
        SafeRelativePath::parse(format!(
            "{}/{}/{}",
            self.group.replace('.', "/"),
            self.artifact,
            self.version
        ))
        .map_err(Error::from)
    }

    pub(crate) fn file_name_for_version(&self, resolved_version: &str) -> Result<String> {
        super::validation::validate_component(
            resolved_version,
            "maven snapshot version",
            resolved_version,
        )?;

        Ok(match self.classifier.as_deref() {
            Some(classifier) => format!(
                "{}-{resolved_version}-{classifier}.{}",
                self.artifact, self.extension
            ),
            None => format!("{}-{resolved_version}.{}", self.artifact, self.extension),
        })
    }

    pub fn artifact_path(&self) -> Result<SafeRelativePath> {
        SafeRelativePath::parse(format!(
            "{}/{}",
            self.version_directory_path()?.as_str(),
            self.file_name_for_version(self.version())?
        ))
        .map_err(Error::from)
    }
}
