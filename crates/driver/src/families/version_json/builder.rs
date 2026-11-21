mod arguments;
mod command;
mod paths;

use elemental_core::{auth::credential::Credential, runtime::distribution::Distribution};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::error::Result;

use super::{
    extensions::PistonMetaDataExt, layout::InstanceStorage, rules::VersionJsonRuleContext,
    variables::LauncherVariables,
};

pub(crate) use paths::resolve_absolute_path;

pub struct VersionJsonLaunchBuilder {
    pub credential: Credential,
    pub runtime: Distribution,
    pub version: InstanceStorage,
    inner: LauncherVariables,
    extra_jvm_arguments: Vec<String>,
    extra_game_arguments: Vec<String>,
}

impl VersionJsonLaunchBuilder {
    pub fn new(
        credential: Credential,
        runtime: Distribution,
        version: InstanceStorage,
        inner: LauncherVariables,
        extra_jvm_arguments: Vec<String>,
        extra_game_arguments: Vec<String>,
    ) -> Self {
        Self {
            credential,
            runtime,
            version,
            inner,
            extra_jvm_arguments,
            extra_game_arguments,
        }
    }

    pub(super) fn build_jvm_arguments(
        &self,
        metadata: &PistonMetaData,
        raw_jvm_arguments: Vec<String>,
    ) -> Result<Vec<String>> {
        arguments::build_jvm_arguments(self, metadata, raw_jvm_arguments)
    }

    pub(super) fn build_game_arguments(
        &self,
        metadata: &PistonMetaData,
        rule_context: &VersionJsonRuleContext,
    ) -> Result<Vec<String>> {
        let mut arguments = self.inner.apply(&metadata.game_arguments(rule_context)?);
        arguments.extend(self.inner.apply(&self.extra_game_arguments));
        Ok(arguments)
    }

    fn build_classpath(
        &self,
        metadata: &PistonMetaData,
        rule_context: &VersionJsonRuleContext,
        module_path_entries: &std::collections::HashSet<String>,
    ) -> Result<String> {
        arguments::build_classpath(self, metadata, rule_context, module_path_entries)
    }

    fn collect_module_path_entries(
        &self,
        jvm_arguments: &[String],
    ) -> Result<std::collections::HashSet<String>> {
        arguments::collect_module_path_entries(self, jvm_arguments)
    }
}
