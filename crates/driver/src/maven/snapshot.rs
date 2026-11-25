#![cfg_attr(not(feature = "driver-all"), allow(dead_code))]

use elemental_schema::maven::{MavenMetadataBody, MavenMetadataVersion, MavenMetadataVersioning};
use quick_xml::{Reader, escape::unescape, events::Event};

use crate::error::{Error, Result};
use crate::http::fetch_text;

use super::{MavenCoordinate, validate_component};

#[derive(Debug, thiserror::Error)]
enum MavenXmlError {
    #[error(transparent)]
    Xml(#[from] quick_xml::Error),
    #[error(transparent)]
    Escape(#[from] quick_xml::escape::EscapeError),
    #[error("unexpected XML end tag </{0}>")]
    UnexpectedEndTag(String),
    #[error("invalid integer '{value}' in <buildNumber>: {source}")]
    InvalidBuildNumber {
        value: String,
        #[source]
        source: std::num::ParseIntError,
    },
    #[error("missing required element '{0}' in Maven metadata")]
    MissingElement(&'static str),
}

#[derive(Debug)]
pub(crate) struct SnapshotMavenMetadata {
    version: String,
    versioning: SnapshotVersioning,
}

#[derive(Debug, Default)]
struct SnapshotVersioning {
    snapshot: Option<SnapshotVersion>,
    snapshot_versions: Vec<SnapshotArtifactVersion>,
}

#[derive(Debug)]
struct SnapshotVersion {
    timestamp: String,
    build_number: usize,
}

#[derive(Debug)]
struct SnapshotArtifactVersion {
    classifier: Option<String>,
    extension: String,
    value: String,
}

#[derive(Debug, Default)]
struct PartialSnapshotArtifact {
    classifier: Option<String>,
    extension: Option<String>,
    value: Option<String>,
}

impl PartialSnapshotArtifact {
    fn finish(self) -> std::result::Result<Option<SnapshotArtifactVersion>, MavenXmlError> {
        if self.classifier.is_none() && self.extension.is_none() && self.value.is_none() {
            return Ok(None);
        }

        let extension = self
            .extension
            .ok_or(MavenXmlError::MissingElement("<extension>"))?;
        let value = self.value.ok_or(MavenXmlError::MissingElement("<value>"))?;
        Ok(Some(SnapshotArtifactVersion {
            classifier: self.classifier,
            extension,
            value,
        }))
    }
}

fn matches_snapshot_artifact(
    coordinate: &MavenCoordinate,
    artifact: &SnapshotArtifactVersion,
) -> bool {
    artifact.extension.as_str() == coordinate.extension()
        && artifact.classifier.as_deref() == coordinate.classifier()
}

pub(crate) fn resolve_snapshot_version(
    coordinate: &MavenCoordinate,
    metadata: &SnapshotMavenMetadata,
) -> Result<String> {
    if let Some(artifact) = metadata
        .versioning
        .snapshot_versions
        .iter()
        .find(|artifact| matches_snapshot_artifact(coordinate, artifact))
    {
        validate_component(
            artifact.value.as_str(),
            "maven snapshot version",
            artifact.value.as_str(),
        )?;
        return Ok(artifact.value.clone());
    }

    if let Some(snapshot) = &metadata.versioning.snapshot
        && !metadata.version.is_empty()
    {
        validate_component(
            metadata.version.as_str(),
            "maven snapshot version",
            metadata.version.as_str(),
        )?;
        validate_component(
            snapshot.timestamp.as_str(),
            "maven snapshot timestamp",
            snapshot.timestamp.as_str(),
        )?;
        return Ok(format!(
            "{}-{}-{}",
            metadata.version, snapshot.timestamp, snapshot.build_number
        ));
    }

    Ok(coordinate.version().to_owned())
}

pub(crate) fn decode_snapshot_metadata(raw: &str, url: String) -> Result<SnapshotMavenMetadata> {
    decode_snapshot_metadata_xml(raw).map_err(|source| Error::DecodeMavenMetadata {
        source_label: "maven snapshot metadata".to_owned(),
        url,
        source: Box::new(source),
    })
}

pub async fn fetch_maven_metadata(
    client: &reqwest::Client,
    url: String,
    source_label: &str,
) -> Result<MavenMetadataBody> {
    let raw = fetch_text(client, url.as_str(), source_label).await?;
    decode_maven_metadata(raw.as_str()).map_err(|source| Error::DecodeMavenMetadata {
        source_label: source_label.to_owned(),
        url,
        source: Box::new(source),
    })
}

fn decode_maven_metadata(raw: &str) -> std::result::Result<MavenMetadataBody, MavenXmlError> {
    let mut saw_versioning = false;
    let mut saw_versions = false;
    let mut versions = Vec::new();

    walk_xml(raw, |event| {
        if let XmlEvent::End(path, text) = event {
            let parts = path.iter().map(String::as_str).collect::<Vec<&str>>();
            match parts.as_slice() {
                ["metadata", "versioning"] => saw_versioning = true,
                ["metadata", "versioning", "versions"] => saw_versions = true,
                ["metadata", "versioning", "versions", "version"] => {
                    versions.push(text.to_owned());
                }
                _ => {}
            }
        }
        Ok(())
    })?;

    if !saw_versioning {
        return Err(MavenXmlError::MissingElement("<versioning>"));
    }
    if !saw_versions {
        return Err(MavenXmlError::MissingElement("<versions>"));
    }

    Ok(MavenMetadataBody {
        versioning: MavenMetadataVersioning {
            versions: MavenMetadataVersion { version: versions },
        },
    })
}

#[derive(Debug, Default)]
struct PendingSnapshot {
    timestamp: Option<String>,
    build_number: Option<usize>,
}

impl PendingSnapshot {
    fn finish(self) -> std::result::Result<SnapshotVersion, MavenXmlError> {
        Ok(SnapshotVersion {
            timestamp: self
                .timestamp
                .ok_or(MavenXmlError::MissingElement("<timestamp>"))?,
            build_number: self
                .build_number
                .ok_or(MavenXmlError::MissingElement("<buildNumber>"))?,
        })
    }
}

#[derive(Debug, Default)]
struct SnapshotParser {
    saw_versioning: bool,
    version: String,
    pending_snapshot: Option<PendingSnapshot>,
    snapshot: Option<SnapshotVersion>,
    current_artifact: Option<PartialSnapshotArtifact>,
    snapshot_versions: Vec<SnapshotArtifactVersion>,
}

impl SnapshotParser {
    fn handle(&mut self, event: XmlEvent<'_>) -> std::result::Result<(), MavenXmlError> {
        match event {
            XmlEvent::Start(path) => {
                let parts = path.iter().map(String::as_str).collect::<Vec<&str>>();
                self.on_start(parts.as_slice());
                Ok(())
            }
            XmlEvent::End(path, text) => {
                let parts = path.iter().map(String::as_str).collect::<Vec<&str>>();
                self.on_end(parts.as_slice(), text)
            }
        }
    }

    fn on_start(&mut self, parts: &[&str]) {
        match parts {
            ["metadata", "versioning", "snapshot"] => {
                self.pending_snapshot = Some(PendingSnapshot::default());
            }
            [
                "metadata",
                "versioning",
                "snapshotVersions",
                "snapshotVersion",
            ] => {
                self.current_artifact = Some(PartialSnapshotArtifact::default());
            }
            _ => {}
        }
    }

    fn on_end(&mut self, parts: &[&str], text: &str) -> std::result::Result<(), MavenXmlError> {
        match parts {
            ["metadata", "version"] => self.version = text.to_owned(),
            ["metadata", "versioning"] => self.saw_versioning = true,
            ["metadata", "versioning", "snapshot"] => {
                if let Some(pending) = self.pending_snapshot.take() {
                    self.snapshot = Some(pending.finish()?);
                }
            }
            ["metadata", "versioning", "snapshot", rest @ ..] => {
                self.on_snapshot_end(rest, text)?;
            }
            ["metadata", "versioning", "snapshotVersions", rest @ ..] => {
                self.on_artifact_end(rest, text)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn on_snapshot_end(
        &mut self,
        rest: &[&str],
        text: &str,
    ) -> std::result::Result<(), MavenXmlError> {
        let Some(pending) = &mut self.pending_snapshot else {
            return Ok(());
        };
        match rest {
            ["timestamp"] => pending.timestamp = Some(text.to_owned()),
            ["buildNumber"] => {
                let parsed =
                    text.parse::<usize>()
                        .map_err(|source| MavenXmlError::InvalidBuildNumber {
                            value: text.to_owned(),
                            source,
                        })?;
                pending.build_number = Some(parsed);
            }
            _ => {}
        }
        Ok(())
    }

    fn on_artifact_end(
        &mut self,
        rest: &[&str],
        text: &str,
    ) -> std::result::Result<(), MavenXmlError> {
        let field = match rest {
            ["snapshotVersion", field] | [field] if *field != "snapshotVersion" => field,
            [] | ["snapshotVersion"] => {
                if let Some(artifact) = self
                    .current_artifact
                    .take()
                    .and_then(|item| item.finish().transpose())
                {
                    self.snapshot_versions.push(artifact?);
                }
                return Ok(());
            }
            _ => return Ok(()),
        };

        let artifact = self.current_artifact.get_or_insert_with(Default::default);
        match *field {
            "classifier" => artifact.classifier = (!text.is_empty()).then(|| text.to_owned()),
            "extension" => artifact.extension = Some(text.to_owned()),
            "value" => artifact.value = Some(text.to_owned()),
            _ => {}
        }
        Ok(())
    }

    fn into_metadata(self) -> std::result::Result<SnapshotMavenMetadata, MavenXmlError> {
        if !self.saw_versioning {
            return Err(MavenXmlError::MissingElement("<versioning>"));
        }
        Ok(SnapshotMavenMetadata {
            version: self.version,
            versioning: SnapshotVersioning {
                snapshot: self.snapshot,
                snapshot_versions: self.snapshot_versions,
            },
        })
    }
}

fn decode_snapshot_metadata_xml(
    raw: &str,
) -> std::result::Result<SnapshotMavenMetadata, MavenXmlError> {
    let mut parser = SnapshotParser::default();
    walk_xml(raw, |event| parser.handle(event))?;
    parser.into_metadata()
}

enum XmlEvent<'a> {
    Start(&'a [String]),
    End(&'a [String], &'a str),
}

fn walk_xml(
    raw: &str,
    mut on_event: impl FnMut(XmlEvent<'_>) -> std::result::Result<(), MavenXmlError>,
) -> std::result::Result<(), MavenXmlError> {
    let mut reader = Reader::from_str(raw);
    reader.config_mut().trim_text(true);
    let mut path = Vec::<String>::new();
    let mut text = String::new();

    loop {
        match reader.read_event()? {
            Event::Start(event) => {
                path.push(event.local_name().as_ref().to_owned());
                text.clear();
                on_event(XmlEvent::Start(&path))?;
            }
            Event::Empty(event) => {
                path.push(event.local_name().as_ref().to_owned());
                text.clear();
                on_event(XmlEvent::Start(&path))?;
                on_event(XmlEvent::End(&path, ""))?;
                path.pop();
            }
            Event::Text(event) => {
                text.push_str(unescape(event.as_ref())?.as_ref());
            }
            Event::CData(event) => {
                text.push_str(event.as_ref());
            }
            Event::End(event) => {
                let local_name = event.local_name();
                let tag = local_name.as_ref();
                if path.last().is_none_or(|open| open.as_str() != tag) {
                    return Err(MavenXmlError::UnexpectedEndTag(tag.to_owned()));
                }
                on_event(XmlEvent::End(&path, text.trim()))?;
                text.clear();
                path.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_release_maven_metadata() {
        let xml = r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <metadata>
              <groupId>net.neoforged</groupId>
              <artifactId>neoforge</artifactId>
              <versioning>
                <latest>21.1.72</latest>
                <release>21.1.72</release>
                <versions>
                  <version>21.1.70</version>
                  <version>21.1.71</version>
                  <version>21.1.72</version>
                </versions>
              </versioning>
            </metadata>
        "#;

        let parsed = decode_maven_metadata(xml).expect("valid release maven metadata");
        assert_eq!(
            parsed.versioning.versions.version,
            vec!["21.1.70", "21.1.71", "21.1.72"]
        );
    }

    #[test]
    fn resolves_snapshot_artifact_from_snapshot_versions() {
        let xml = r#"
            <metadata>
              <version>1.12.2-SNAPSHOT</version>
              <versioning>
                <snapshot>
                  <timestamp>20180509.180000</timestamp>
                  <buildNumber>4</buildNumber>
                </snapshot>
                <snapshotVersions>
                  <snapshotVersion>
                    <classifier>javadoc</classifier>
                    <extension>jar</extension>
                    <value>1.12.2-20180509.180000-4</value>
                  </snapshotVersion>
                  <snapshotVersion>
                    <extension>jar</extension>
                    <value>1.12.2-20180509.180000-4</value>
                  </snapshotVersion>
                </snapshotVersions>
              </versioning>
            </metadata>
        "#;

        let coord = MavenCoordinate::parse("com.mumfrey:liteloader:1.12.2-SNAPSHOT").unwrap();
        let metadata = decode_snapshot_metadata(xml, "https://example.invalid/meta.xml".to_owned())
            .expect("valid snapshot metadata");
        let resolved =
            resolve_snapshot_version(&coord, &metadata).expect("resolve snapshot version");
        assert_eq!(resolved, "1.12.2-20180509.180000-4");
    }

    #[test]
    fn resolves_snapshot_artifact_from_timestamp_fallback() {
        let xml = r#"
            <metadata>
              <version>1.12.2-SNAPSHOT</version>
              <versioning>
                <snapshot>
                  <timestamp>20190101.120000</timestamp>
                  <buildNumber>7</buildNumber>
                </snapshot>
              </versioning>
            </metadata>
        "#;

        let coord = MavenCoordinate::parse("com.mumfrey:liteloader:1.12.2-SNAPSHOT").unwrap();
        let metadata = decode_snapshot_metadata(xml, "https://example.invalid/meta.xml".to_owned())
            .expect("valid fallback snapshot metadata");
        let resolved =
            resolve_snapshot_version(&coord, &metadata).expect("resolve snapshot version");
        assert_eq!(resolved, "1.12.2-SNAPSHOT-20190101.120000-7");
    }

    #[test]
    fn rejects_malformed_or_non_metadata_xml() {
        assert!(decode_maven_metadata("<metadata></metadata>").is_err());
        assert!(matches!(
            decode_snapshot_metadata("<broken>", "https://example.invalid/meta.xml".to_owned()),
            Err(Error::DecodeMavenMetadata { .. })
        ));
    }
}
