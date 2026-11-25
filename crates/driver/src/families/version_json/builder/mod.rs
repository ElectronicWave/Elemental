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

    pub(super) fn build_game_arguments(
        &self,
        metadata: &PistonMetaData,
        rule_context: &VersionJsonRuleContext,
    ) -> Result<Vec<String>> {
        let mut arguments = self.inner.apply(&metadata.game_arguments(rule_context)?);
        arguments.extend(self.inner.apply(&self.extra_game_arguments));
        Ok(arguments)
    }
}
