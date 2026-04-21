use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use elemental_core::{
    auth::authorizer::Authorizer, launcher::command::LaunchCommand,
    runtime::distribution::Distribution, storage::Storage,
};

use crate::launch_arguments::parse_argument_string;

use super::{
    classpath::{classpath_separator, join_classpath},
    extensions::{PistonMetaDataExt, PistonMetaLibrariesExt},
    layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    rules::VersionJsonRuleContext,
    storage::{VersionJsonGameStorageExt, VersionJsonVersionStorageExt},
    variables::LauncherVariables,
};

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

    pub fn try_set_extra_jvm_argument_string(
        mut self,
        extra_jvm_argument_string: String,
    ) -> Result<Self> {
        self.extra_jvm_arguments = parse_argument_string(extra_jvm_argument_string.as_str())?;
        Ok(self)
    }

    pub fn try_set_extra_game_argument_string(
        mut self,
        extra_game_argument_string: String,
    ) -> Result<Self> {
        self.extra_game_arguments = parse_argument_string(extra_game_argument_string.as_str())?;
        Ok(self)
    }

    pub async fn build_command(mut self) -> Result<LaunchCommand> {
        let version_name = self.version.name().context("get version name failed")?;
        let version_root = self.version.path.clone();
        let global_root = self.version.parent.path.clone();
        let version_jar = self.version.jar_path()?;

        if !version_jar.exists() {
            bail!("version jar not found: {}", version_jar.to_string_lossy());
        }

        let metadata = self
            .version
            .metadata()
            .context("read version metadata failed")?;
        let rule_context = VersionJsonRuleContext::current();
        let credential = self
            .authorizer
            .authorize()
            .await
            .context("authorize failed")?;

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
        self.inner.game_directory = version_root.to_string_lossy().to_string();
        self.inner.assets_root = global_root.join("assets").to_string_lossy().to_string();
        self.inner.assets_index_name = metadata.assets.clone();
        self.inner.auth_uuid = credential.uuid;
        self.inner.auth_access_token = credential.access_token;
        self.inner.user_type = if self.inner.auth_access_token.is_empty() {
            crate::families::version_json::variables::UserType::Legacy
        } else {
            crate::families::version_json::variables::UserType::Msa
        };
        self.inner.version_type = metadata.release_type.clone();
        self.inner.library_directory = global_root.join("libraries").to_string_lossy().to_string();
        self.inner.classpath_separator = classpath_separator().to_owned();
        self.inner.natives_directory = self
            .version
            .platform_natives_path()
            .to_string_lossy()
            .to_string();

        let classpath = metadata
            .libraries
            .iter()
            .filter(|library| library.is_allowed(&rule_context))
            .filter_map(|library| {
                let artifact = library.downloads.artifact.as_ref()?;
                if artifact.path.contains("natives") {
                    return None;
                }

                Some(
                    global_root
                        .join("libraries")
                        .join(&artifact.path)
                        .to_string_lossy()
                        .to_string(),
                )
            })
            .chain(std::iter::once(version_jar.to_string_lossy().to_string()))
            .collect::<Vec<String>>();
        self.inner.classpath = join_classpath(classpath);

        let mut jvm_args = Vec::new();
        if metadata.arguments.is_none() {
            jvm_args.extend(self.inner.apply(legacy_jvm_arguments())?);
        }

        if let Some(logging) = &metadata.logging
            && let Some(client) = &logging.client
        {
            let log_config_path = self.version.parent.logging_config_path(&client.file.id)?;

            if !log_config_path.exists() {
                bail!(
                    "logging config not found: {}",
                    log_config_path.to_string_lossy()
                );
            }

            jvm_args.extend(self.inner.apply_with(
                vec![client.argument.clone()],
                &HashMap::from([(
                    "path".to_owned(),
                    log_config_path.to_string_lossy().to_string(),
                )]),
            )?);
        }

        jvm_args.extend(self.inner.apply(metadata.jvm_arguments(&rule_context))?);
        jvm_args.extend(self.inner.apply(self.extra_jvm_arguments)?);
        let mut game_args = self.inner.apply(metadata.game_arguments(&rule_context))?;
        game_args.extend(self.inner.apply(self.extra_game_arguments)?);

        let mut args = jvm_args;
        args.push(metadata.main_class);
        args.extend(game_args);

        Ok(LaunchCommand::new(self.runtime.executable(), args))
    }

    pub fn variables(self) -> LauncherVariables {
        self.inner
    }
}

fn legacy_jvm_arguments() -> Vec<String> {
    vec![
        "-Djava.library.path=${natives_directory}".to_owned(),
        "-Dminecraft.launcher.brand=${launcher_name}".to_owned(),
        "-Dminecraft.launcher.version=${launcher_version}".to_owned(),
        "-cp".to_owned(),
        "${classpath}".to_owned(),
    ]
}
