mod arguments;
mod command;
mod paths;

use elemental_core::{
    auth::credential::UserCredential, runtime::distribution::Distribution, storage::Storage,
};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::error::Result;

use super::{
    extensions::PistonMetaDataExt,
    layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    rules::VersionJsonRuleContext,
    variables::LauncherVariables,
};

use paths::LaunchPaths;

pub struct VersionJsonLaunchBuilder<L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> {
    pub credential: UserCredential,
    pub runtime: Distribution,
    pub version: Storage<VL, Storage<L>>,
    inner: LauncherVariables,
    extra_jvm_arguments: Vec<String>,
    extra_game_arguments: Vec<String>,
}

impl<L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> VersionJsonLaunchBuilder<L, VL> {
    pub fn new(
        credential: UserCredential,
        runtime: Distribution,
        version: Storage<VL, Storage<L>>,
    ) -> Self {
        Self {
            credential,
            runtime,
            version,
            inner: LauncherVariables::default(),
            extra_jvm_arguments: Vec::new(),
            extra_game_arguments: Vec::new(),
        }
    }

    pub fn set_quick_play(
        mut self,
        quick_play_path: Option<String>,
        quick_play_multiplayer: Option<String>,
        quick_play_singleplayer: Option<String>,
        quick_play_realms: Option<String>,
    ) -> Self {
        self.inner.quick_play_path = quick_play_path;
        self.inner.quick_play_multiplayer = quick_play_multiplayer;
        self.inner.quick_play_singleplayer = quick_play_singleplayer;
        self.inner.quick_play_realms = quick_play_realms;
        self
    }

    pub fn set_username(mut self, username: String) -> Self {
        self.inner.auth_player_name = username;
        self
    }

    pub fn set_resolution(mut self, width: String, height: String) -> Self {
        self.inner.resolution_width = width;
        self.inner.resolution_height = height;
        self
    }

    pub fn set_client_id(mut self, client_id: String) -> Self {
        self.inner.clientid = client_id;
        self
    }

    pub fn set_launcher(mut self, name: String, version: String) -> Self {
        self.inner.launcher_name = name;
        self.inner.launcher_version = version;
        self
    }

    pub fn set_extra_jvm_arguments(mut self, extra_jvm_arguments: Vec<String>) -> Self {
        self.extra_jvm_arguments = extra_jvm_arguments;
        self
    }

    pub fn set_extra_game_arguments(mut self, extra_game_arguments: Vec<String>) -> Self {
        self.extra_game_arguments = extra_game_arguments;
        self
    }

    pub fn variables(self) -> LauncherVariables {
        self.inner
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
        let mut arguments = self.inner.apply(metadata.game_arguments(rule_context))?;
        arguments.extend(self.inner.apply(self.extra_game_arguments.clone())?);
        Ok(arguments)
    }

    fn build_classpath(
        &self,
        metadata: &PistonMetaData,
        rule_context: &VersionJsonRuleContext,
        paths: &LaunchPaths,
        module_path_entries: &std::collections::HashSet<String>,
    ) -> Result<String> {
        arguments::build_classpath(self, metadata, rule_context, paths, module_path_entries)
    }

    fn collect_module_path_entries(
        &self,
        jvm_arguments: &[String],
    ) -> Result<std::collections::HashSet<String>> {
        arguments::collect_module_path_entries(self, jvm_arguments)
    }
}
