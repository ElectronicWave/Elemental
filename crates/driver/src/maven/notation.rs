use crate::error::{Error, Result};

use super::validate_component;

/// Maven coordinate in launcher notation:
/// `group:artifact:version[:classifier][@extension]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenNotation {
    pub(crate) group: String,
    pub(crate) artifact: String,
    pub(crate) version: String,
    pub(crate) classifier: Option<String>,
    pub(crate) extension: String,
}

impl MavenNotation {
    pub fn parse(notation: &str, invalid_label: &str) -> Result<Self> {
        let (coordinates, extension) = notation.split_once('@').unwrap_or((notation, "jar"));
        let mut segments = coordinates.splitn(4, ':');
        let (Some(group), Some(artifact), Some(version)) =
            (segments.next(), segments.next(), segments.next())
        else {
            return Err(Error::InvalidNotation {
                label: invalid_label.to_owned(),
                notation: notation.to_owned(),
            });
        };
        let classifier = segments.next();

        validate_component(group, invalid_label, notation)?;
        validate_component(artifact, invalid_label, notation)?;
        validate_component(version, invalid_label, notation)?;
        validate_component(extension, invalid_label, notation)?;
        if let Some(classifier) = classifier {
            validate_component(classifier, invalid_label, notation)?;
        }

        Ok(Self {
            group: group.to_owned(),
            artifact: artifact.to_owned(),
            version: version.to_owned(),
            classifier: classifier.map(ToOwned::to_owned),
            extension: extension.to_owned(),
        })
    }

    pub fn group(&self) -> &str {
        self.group.as_str()
    }

    pub(crate) fn version(&self) -> &str {
        self.version.as_str()
    }

    pub fn with_classifier(&self, classifier: &str) -> Result<Self> {
        if classifier.is_empty() {
            return Err(Error::InvalidNotation {
                label: "maven classifier".to_owned(),
                notation: classifier.to_owned(),
            });
        }
        validate_component(classifier, "maven classifier", classifier)?;

        Ok(Self {
            group: self.group.clone(),
            artifact: self.artifact.clone(),
            version: self.version.clone(),
            classifier: Some(classifier.to_owned()),
            extension: self.extension.clone(),
        })
    }
}
