use std::fmt::{self, Display, Formatter};

use elemental_infra::SafeRelativePath;

use crate::error::{Error, Result};

/// Maven coordinate in launcher notation:
/// `group:artifact:version[:classifier][@extension]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenCoordinate {
    group: String,
    artifact: String,
    version: String,
    classifier: Option<String>,
    extension: String,
}

impl MavenCoordinate {
    pub fn parse(notation: &str) -> Option<Self> {
        let (coordinates, extension) = notation.split_once('@').unwrap_or((notation, "jar"));
        let mut segments = coordinates.splitn(4, ':');
        let (Some(group), Some(artifact), Some(version)) =
            (segments.next(), segments.next(), segments.next())
        else {
            return None;
        };
        let classifier = segments.next();

        if [group, artifact, version, extension]
            .iter()
            .any(|component| !is_valid_component(component))
            || classifier.is_some_and(|classifier| !is_valid_component(classifier))
        {
            return None;
        }

        Some(Self {
            group: group.to_owned(),
            artifact: artifact.to_owned(),
            version: version.to_owned(),
            classifier: classifier.map(ToOwned::to_owned),
            extension: extension.to_owned(),
        })
    }

    pub fn parse_labeled(notation: &str, label: &str) -> Result<Self> {
        Self::parse(notation).ok_or_else(|| Error::InvalidNotation {
            label: label.to_owned(),
            notation: notation.to_owned(),
        })
    }

    pub fn group(&self) -> &str {
        self.group.as_str()
    }

    pub fn artifact(&self) -> &str {
        self.artifact.as_str()
    }

    pub fn version(&self) -> &str {
        self.version.as_str()
    }

    pub fn classifier(&self) -> Option<&str> {
        self.classifier.as_deref()
    }

    pub fn extension(&self) -> &str {
        self.extension.as_str()
    }

    pub fn with_classifier(&self, classifier: &str) -> Option<Self> {
        is_valid_component(classifier).then(|| Self {
            classifier: Some(classifier.to_owned()),
            ..self.clone()
        })
    }

    pub fn version_directory_path(&self) -> String {
        format!("{}/{}", self.artifact_directory_path(), self.version)
    }

    pub fn artifact_directory_path(&self) -> String {
        format!("{}/{}", self.group.replace('.', "/"), self.artifact)
    }

    pub fn artifact_directory_path_for(group: &str, artifact: &str) -> Option<String> {
        (is_valid_component(group) && is_valid_component(artifact))
            .then(|| format!("{}/{artifact}", group.replace('.', "/")))
    }

    pub fn file_name(&self, version: &str) -> String {
        match self.classifier.as_deref() {
            Some(classifier) => format!(
                "{}-{version}-{classifier}.{}",
                self.artifact, self.extension
            ),
            None => format!("{}-{version}.{}", self.artifact, self.extension),
        }
    }

    pub fn artifact_path_for_version(&self, version: &str) -> String {
        format!(
            "{}/{}",
            self.version_directory_path(),
            self.file_name(version)
        )
    }

    pub fn artifact_path(&self) -> String {
        self.artifact_path_for_version(self.version.as_str())
    }

    pub fn metadata_path(&self) -> String {
        format!("{}/maven-metadata.xml", self.version_directory_path())
    }

    pub fn safe_artifact_path(&self) -> Result<SafeRelativePath> {
        SafeRelativePath::parse(self.artifact_path()).map_err(Error::from)
    }

    pub fn safe_version_directory(&self) -> Result<SafeRelativePath> {
        SafeRelativePath::parse(self.version_directory_path()).map_err(Error::from)
    }

    pub fn safe_artifact_path_for_version(&self, version: &str) -> Result<SafeRelativePath> {
        super::validate_component(version, "maven snapshot version", version)?;

        SafeRelativePath::parse(self.artifact_path_for_version(version)).map_err(Error::from)
    }

    pub fn safe_metadata_path(&self) -> Result<SafeRelativePath> {
        SafeRelativePath::parse(self.metadata_path()).map_err(Error::from)
    }
}

impl Display for MavenCoordinate {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{}",
            self.group, self.artifact, self.version
        )?;
        if let Some(classifier) = &self.classifier {
            write!(formatter, ":{classifier}")?;
        }
        if self.extension != "jar" {
            write!(formatter, "@{}", self.extension)?;
        }
        Ok(())
    }
}

pub(super) fn is_valid_component(value: &str) -> bool {
    let rejected = value.is_empty()
        || value == "."
        || value == ".."
        || value.contains(['/', '\\', ':', '?', '#'])
        || value.chars().any(char::is_control);

    !rejected
}
