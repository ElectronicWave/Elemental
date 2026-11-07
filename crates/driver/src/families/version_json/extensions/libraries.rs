use elemental_schema::mojang::piston::{PistonMetaLibraries, PistonMetaLibrariesDownloadsArtifact};

use super::super::{PistonMetaRulesExt, VersionJsonPlatform, VersionJsonRuleContext};

pub trait PistonMetaLibrariesExt {
    fn is_allowed(&self, context: &VersionJsonRuleContext) -> bool;
    fn version_artifacts<'a>(
        &'a self,
        platform: &VersionJsonPlatform,
    ) -> [Option<&'a PistonMetaLibrariesDownloadsArtifact>; 2];
    fn native_source_artifacts<'a>(
        &'a self,
        platform: &VersionJsonPlatform,
    ) -> [Option<&'a PistonMetaLibrariesDownloadsArtifact>; 2];
    fn classifiers_native_artifact<'a>(
        &'a self,
        platform: &VersionJsonPlatform,
    ) -> Option<&'a PistonMetaLibrariesDownloadsArtifact>;
    fn native_artifact<'a>(
        &'a self,
        platform: &VersionJsonPlatform,
    ) -> Option<&'a PistonMetaLibrariesDownloadsArtifact>;
}

impl PistonMetaLibrariesExt for PistonMetaLibraries {
    fn is_allowed(&self, context: &VersionJsonRuleContext) -> bool {
        self.rules
            .as_deref()
            .map(|rules| rules.are_allowed(context))
            .unwrap_or(true)
    }

    fn version_artifacts<'a>(
        &'a self,
        platform: &VersionJsonPlatform,
    ) -> [Option<&'a PistonMetaLibrariesDownloadsArtifact>; 2] {
        [
            self.downloads.artifact.as_ref(),
            self.classifiers_native_artifact(platform),
        ]
    }

    fn native_source_artifacts<'a>(
        &'a self,
        platform: &VersionJsonPlatform,
    ) -> [Option<&'a PistonMetaLibrariesDownloadsArtifact>; 2] {
        [
            self.classifiers_native_artifact(platform),
            self.native_artifact(platform),
        ]
    }

    fn classifiers_native_artifact<'a>(
        &'a self,
        platform: &VersionJsonPlatform,
    ) -> Option<&'a PistonMetaLibrariesDownloadsArtifact> {
        let classifiers = self.downloads.classifiers.as_ref()?;
        let natives = self.natives.as_ref()?;

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
        &'a self,
        platform: &VersionJsonPlatform,
    ) -> Option<&'a PistonMetaLibrariesDownloadsArtifact> {
        let artifact = self.downloads.artifact.as_ref()?;
        if artifact
            .path
            .ends_with(&format!("-natives-{}.jar", platform.os()))
            || platform.os() == "macos" && artifact.path.ends_with("-natives-osx.jar")
        {
            return Some(artifact);
        }

        None
    }
}
