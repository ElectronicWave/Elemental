mod arguments;
mod command;
mod paths;

use elemental_core::{
    auth::credential::Credential, runtime::distribution::Distribution, storage::Storage,
};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::error::Result;

use super::{
    extensions::PistonMetaDataExt,
    layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    rules::VersionJsonRuleContext,
    variables::LauncherVariables,
};

pub(crate) use paths::resolve_absolute_path;

pub struct VersionJsonLaunchBuilder<L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> {
    pub credential: Credential,
    pub runtime: Distribution,
    pub version: Storage<VL, Storage<L>>,
    inner: LauncherVariables,
    extra_jvm_arguments: Vec<String>,
    extra_game_arguments: Vec<String>,
}

impl<L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> VersionJsonLaunchBuilder<L, VL> {
    pub fn new(
        credential: Credential,
        runtime: Distribution,
        version: Storage<VL, Storage<L>>,
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
        let mut arguments = self.inner.apply(metadata.game_arguments(rule_context)?)?;
        arguments.extend(self.inner.apply(self.extra_game_arguments.clone())?);
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
