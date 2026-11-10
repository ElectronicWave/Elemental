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
    fn downloads(&self) -> Result<Cow<'_, PistonMetaLibrariesDownloads>> {
        self.downloads_for(&VersionJsonPlatform::current())
    }
    fn downloads_for(
        &self,
        platform: &VersionJsonPlatform,
    ) -> Result<Cow<'_, PistonMetaLibrariesDownloads>>;
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

    fn downloads_for(
        &self,
        platform: &VersionJsonPlatform,
    ) -> Result<Cow<'_, PistonMetaLibrariesDownloads>> {
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
                    platform.arch_bits(),
                )
                .map(Cow::Owned)
            }
        }
    }

    fn version_artifacts(
        &self,
        platform: &VersionJsonPlatform,
    ) -> Result<Vec<PistonMetaLibrariesDownloadsArtifact>> {
        let downloads = self.downloads_for(platform)?;
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
        let downloads = self.downloads_for(platform)?;
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
    let descriptors = platform.native_descriptors();

    if let Some(natives) = natives {
        for descriptor in &descriptors {
            let Some(classifier) = natives.get(descriptor) else {
                continue;
            };
            let classifier = classifier.replace("${arch}", platform.arch_bits());
            if let Some(artifact) = classifiers.get(&classifier) {
                return Some(artifact);
            }
        }
    }

    for descriptor in &descriptors {
        if let Some(artifact) = classifiers.get(descriptor) {
            return Some(artifact);
        }
    }

    None
}

fn native_artifact<'a>(
    downloads: &'a PistonMetaLibrariesDownloads,
    platform: &VersionJsonPlatform,
) -> Option<&'a PistonMetaLibrariesDownloadsArtifact> {
    let artifact = downloads.artifact.as_ref()?;
    if platform
        .native_os_names()
        .iter()
        .any(|os| artifact.path.ends_with(&format!("-natives-{os}.jar")))
    {
        return Some(artifact);
    }

    None
}
