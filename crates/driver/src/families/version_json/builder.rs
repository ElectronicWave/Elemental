mod arguments;
mod paths;

use elemental_core::{
    auth::{authorizer::Authorizer, credential::UserCredential},
    launcher::command::LaunchCommand,
    runtime::distribution::Distribution,
    storage::Storage,
};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::error::{Error, Result};

use super::{
    classpath::classpath_separator,
    extensions::PistonMetaDataExt,
    layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    rules::VersionJsonRuleContext,
    storage::VersionJsonVersionStorageExt,
    variables::{LauncherVariables, UserType},
};

use paths::LaunchPaths;

pub struct VersionJsonLaunchBuilder<
    A: Authorizer,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
> {
    pub authorizer: A,
    pub runtime: Distribution,
    pub version: Storage<VL, Storage<L>>,
    inner: LauncherVariables,
    extra_jvm_arguments: Vec<String>,
    extra_game_arguments: Vec<String>,
}

impl<A: Authorizer, L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout>
    VersionJsonLaunchBuilder<A, L, VL>
{
    pub fn new(authorizer: A, runtime: Distribution, version: Storage<VL, Storage<L>>) -> Self {
        Self {
            authorizer,
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

    pub async fn build_command(mut self) -> Result<LaunchCommand> {
        let metadata = self
            .version
            .metadata()
            .map_err(|source| Error::ReadVersionMetadata {
                source: Box::new(source),
            })?;
        let rule_context = VersionJsonRuleContext::current();
        let credential = self
            .authorizer
            .authorize()
            .await
            .map_err(|source| Error::Authorize { source })?;
        let version_name = self
            .version
            .name()
            .ok_or_else(|| Error::MissingVersionName {
                path: self.version.path.clone(),
            })?;
        let paths = LaunchPaths::resolve(&self.version)?;
        paths.ensure_version_jar_exists()?;

        self.apply_default_variables(&metadata, credential, version_name, &paths);

        let raw_jvm_arguments = metadata.jvm_arguments(&rule_context);
        let module_path_entries = self.collect_module_path_entries(raw_jvm_arguments.as_slice())?;
        self.inner.classpath =
            self.build_classpath(&metadata, &rule_context, &paths, &module_path_entries)?;

        let command_arguments =
            self.build_command_arguments(&metadata, &rule_context, raw_jvm_arguments)?;

        Ok(
            LaunchCommand::new(self.runtime.executable(), command_arguments)
                .with_cwd(paths.version_root),
        )
    }

    pub fn variables(self) -> LauncherVariables {
        self.inner
    }

    fn apply_default_variables(
        &mut self,
        metadata: &PistonMetaData,
        credential: UserCredential,
        version_name: String,
        paths: &LaunchPaths,
    ) {
        if self.inner.auth_player_name.is_empty() {
            self.inner.auth_player_name = credential.username.clone();
        }
        if self.inner.resolution_width.is_empty() {
            self.inner.resolution_width = "854".to_owned();
        }
        if self.inner.resolution_height.is_empty() {
            self.inner.resolution_height = "480".to_owned();
        }
        if self.inner.launcher_name.is_empty() {
            self.inner.launcher_name = "Elemental".to_owned();
        }
        if self.inner.launcher_version.is_empty() {
            self.inner.launcher_version = env!("CARGO_PKG_VERSION").to_owned();
        }

        self.inner.version_name = version_name;
        self.inner.game_directory = paths.version_root.to_string_lossy().to_string();
        self.inner.assets_root = paths.assets_root.to_string_lossy().to_string();
        self.inner.assets_index_name = metadata.assets.clone();
        self.inner.auth_uuid = credential.uuid;
        self.inner.auth_access_token = credential.access_token;
        self.inner.user_type = if self.inner.auth_access_token.is_empty() {
            UserType::Legacy
        } else {
            UserType::Msa
        };
        self.inner.version_type = metadata.release_type.clone();
        self.inner.library_directory = paths.libraries_root.to_string_lossy().to_string();
        self.inner.classpath_separator = classpath_separator().to_owned();
        self.inner.natives_directory = paths.natives_directory.to_string_lossy().to_string();
    }

    pub(super) fn build_command_arguments(
        &self,
        metadata: &PistonMetaData,
        rule_context: &VersionJsonRuleContext,
        raw_jvm_arguments: Vec<String>,
    ) -> Result<Vec<String>> {
        let mut arguments = self.build_jvm_arguments(metadata, raw_jvm_arguments)?;
        arguments.push(metadata.main_class.clone());
        arguments.extend(self.inner.apply(metadata.game_arguments(rule_context))?);
        arguments.extend(self.inner.apply(self.extra_game_arguments.clone())?);
        Ok(arguments)
    }

    fn build_jvm_arguments(
        &self,
        metadata: &PistonMetaData,
        raw_jvm_arguments: Vec<String>,
    ) -> Result<Vec<String>> {
        arguments::build_jvm_arguments(self, metadata, raw_jvm_arguments)
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
