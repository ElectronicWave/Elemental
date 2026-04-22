use elemental_schema::mojang::piston::{
    ContinuousArgument, PistonMetaData, PistonMetaGenericArgument, PistonMetaLibraries,
    PistonMetaLibrariesDownloadsArtifact, PistonMetaRuleArgument,
};

use super::{PistonMetaRulesExt, VersionJsonPlatform, VersionJsonRuleContext};

pub trait PistonMetaDataExt {
    fn jvm_arguments(&self, context: &VersionJsonRuleContext) -> Vec<String>;
    fn game_arguments(&self, context: &VersionJsonRuleContext) -> Vec<String>;
}

impl PistonMetaDataExt for PistonMetaData {
    fn jvm_arguments(&self, context: &VersionJsonRuleContext) -> Vec<String> {
        self.arguments
            .as_ref()
            .map(|arguments| collect_arguments(&arguments.jvm, context))
            .unwrap_or_default()
    }

    fn game_arguments(&self, context: &VersionJsonRuleContext) -> Vec<String> {
        if let Some(arguments) = &self.arguments {
            let game_arguments = collect_arguments(&arguments.game, context);
            if !game_arguments.is_empty() {
                return game_arguments;
            }
        }

        self.minecraft_arguments
            .as_deref()
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    }
}

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

fn collect_arguments(
    arguments: &[PistonMetaGenericArgument],
    context: &VersionJsonRuleContext,
) -> Vec<String> {
    let mut result = Vec::new();

    for argument in arguments {
        match argument {
            PistonMetaGenericArgument::Plain(value) => {
                result.push(value.clone());
            }
            PistonMetaGenericArgument::Rule(rule_argument) => {
                if !rule_argument.rules.as_slice().are_allowed(context) {
                    continue;
                }

                extend_argument_values(&mut result, rule_argument);
            }
        }
    }

    result
}

fn extend_argument_values(target: &mut Vec<String>, argument: &PistonMetaRuleArgument) {
    if let Some(value) = &argument.value {
        match value {
            ContinuousArgument::Single(value) => target.push(value.clone()),
            ContinuousArgument::Multi(values) => target.extend(values.iter().cloned()),
        }
    }
}
