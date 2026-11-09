use std::{borrow::Cow, collections::HashMap};

use elemental_schema::mojang::piston::{
    PistonMetaLibraries, PistonMetaLibrariesDownloads, PistonMetaLibrariesDownloadsArtifact,
    PistonMetaLibrariesSource,
};

use crate::{
    error::{Error, Result},
    maven::{MavenCoordinate, maven_library_downloads},
};

use super::super::{PistonMetaRulesExt, VersionJsonPlatform, VersionJsonRuleContext};

pub trait PistonMetaLibrariesExt {
    fn is_allowed(&self, context: &VersionJsonRuleContext) -> bool;
    fn downloads(&self) -> Result<Cow<'_, PistonMetaLibrariesDownloads>>;
    fn version_artifacts(
        &self,
        platform: &VersionJsonPlatform,
    ) -> Result<Vec<PistonMetaLibrariesDownloadsArtifact>>;
    fn native_source_artifacts(
        &self,
        platform: &VersionJsonPlatform,
    ) -> Result<Vec<PistonMetaLibrariesDownloadsArtifact>>;
}

impl PistonMetaLibrariesExt for PistonMetaLibraries {
    fn is_allowed(&self, context: &VersionJsonRuleContext) -> bool {
        self.rules
            .as_deref()
            .map(|rules| rules.are_allowed(context))
            .unwrap_or(true)
    }

    fn downloads(&self) -> Result<Cow<'_, PistonMetaLibrariesDownloads>> {
        match &self.source {
            PistonMetaLibrariesSource::Downloads { downloads, .. } => Ok(Cow::Borrowed(downloads)),
            PistonMetaLibrariesSource::Maven {
                url, sha1, size, ..
            } => {
                let repository_url =
                    url.as_deref()
                        .filter(|url| !url.is_empty())
                        .ok_or_else(|| Error::MavenLibraryMissingSource {
                            notation: self.name.clone(),
                        })?;
                let coordinate = MavenCoordinate::parse_labeled(&self.name, "maven library")?;

                maven_library_downloads(
                    &coordinate,
                    repository_url,
                    sha1.clone(),
                    *size,
                    self.natives.as_ref(),
                )
                .map(Cow::Owned)
            }
        }
    }

    fn version_artifacts(
        &self,
        platform: &VersionJsonPlatform,
    ) -> Result<Vec<PistonMetaLibrariesDownloadsArtifact>> {
        let downloads = self.downloads()?;
        Ok(downloads
            .artifact
            .iter()
            .chain(classifiers_native_artifact(
                &downloads,
                self.natives.as_ref(),
                platform,
            ))
            .cloned()
            .collect())
    }

    fn native_source_artifacts(
        &self,
        platform: &VersionJsonPlatform,
    ) -> Result<Vec<PistonMetaLibrariesDownloadsArtifact>> {
        let downloads = self.downloads()?;
        Ok(
            classifiers_native_artifact(&downloads, self.natives.as_ref(), platform)
                .into_iter()
                .chain(native_artifact(&downloads, platform))
                .cloned()
                .collect(),
        )
    }
}

fn classifiers_native_artifact<'a>(
    downloads: &'a PistonMetaLibrariesDownloads,
    natives: Option<&HashMap<String, String>>,
    platform: &VersionJsonPlatform,
) -> Option<&'a PistonMetaLibrariesDownloadsArtifact> {
    let classifiers = downloads.classifiers.as_ref()?;
    let natives = natives?;

    if let Some(key) = natives.get(platform.os()) {
        return classifiers.get(key);
    }

    if platform.os() == "macos"
        && let Some(key) = natives.get("osx")
    {
        return classifiers.get(key);
    }

    None
}

fn native_artifact<'a>(
    downloads: &'a PistonMetaLibrariesDownloads,
    platform: &VersionJsonPlatform,
) -> Option<&'a PistonMetaLibrariesDownloadsArtifact> {
    let artifact = downloads.artifact.as_ref()?;
    if artifact
        .path
        .ends_with(&format!("-natives-{}.jar", platform.os()))
        || platform.os() == "macos" && artifact.path.ends_with("-natives-osx.jar")
    {
        return Some(artifact);
    }

    None
}
