use anyhow::{Context, Result, bail};

use super::model::LaunchEnvs;
use crate::{
    auth::authorizer::Authorizer,
    consts::PLATFORM_NATIVES_DIR_NAME,
    runtime::distribution::Distribution,
    storage::{
        layout::{Layout, Layoutable},
        resource::Resource,
        version::VersionStorage,
    },
};

pub struct LaunchBuilder<A: Authorizer, L: Layout, VL: Layout> {
    pub authorizer: A,
    pub runtime: Distribution,
    pub version: VersionStorage<L, VL>,
    inner: LaunchEnvs,
}

impl<A: Authorizer, L: Layout, VL: Layout> LaunchBuilder<A, L, VL> {
    pub fn new(authorizer: A, runtime: Distribution, version: VersionStorage<L, VL>) -> Self {
        Self {
            authorizer,
            runtime,
            version,
            inner: LaunchEnvs::default(),
        }
    }

    pub fn set_quick_play_path(
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

    pub async fn build(mut self) -> Result<Vec<String>> {
        let version_name = self.version.name().context("get version name failed")?;
        let version_root = self
            .version
            .get_existing_resource(Resource::Dot)
            .context("get version root failed")?;
        let global_root = self
            .version
            .global
            .get_existing_resource(Resource::Dot)
            .context("get game root failed")?;
        let versionjar = self.version.jar_path()?;

        if !versionjar.exists() {
            bail!("version jar not found: {}", versionjar.to_string_lossy());
        }

        let metadata = self
            .version
            .metadata()
            .context("read version metadata failed")?;
        let credential = self
            .authorizer
            .authorize()
            .await
            .context("authorize failed")?;

        if self.inner.auth_player_name.is_empty() {
            self.inner.auth_player_name = version_name.clone();
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
        self.inner.version_type = metadata.release_type.clone();
        self.inner.natives_directory = version_root
            .join(PLATFORM_NATIVES_DIR_NAME)
            .to_string_lossy()
            .to_string();

        let classpath = metadata
            .libraries
            .iter()
            .filter_map(|library| {
                if library.downloads.artifact.path.contains("natives") {
                    None
                } else {
                    Some(
                        global_root
                            .join("libraries")
                            .join(&library.downloads.artifact.path)
                            .to_string_lossy()
                            .to_string(),
                    )
                }
            })
            .collect::<Vec<String>>()
            .join(";")
            + ";"
            + &versionjar.to_string_lossy();
        self.inner.classpath = classpath;

        let jvm_args = self
            .inner
            .apply_launchenvs(metadata.arguments.get_jvm_arguments())?;
        let game_args = if !metadata.arguments.game.is_empty() {
            self.inner
                .apply_launchenvs(metadata.arguments.get_game_arguments())?
        } else {
            self.inner.apply_launchenvs(
                metadata
                    .minecraft_arguments
                    .unwrap_or_default()
                    .split_whitespace()
                    .map(|v| v.to_string())
                    .collect(),
            )?
        };

        let mut args = vec![self.runtime.executable().to_string_lossy().to_string()];
        args.extend(jvm_args);
        args.push(metadata.main_class);
        args.extend(game_args);

        Ok(args)
    }

    pub async fn launch(self) -> Result<tokio::process::Child> {
        super::process::spawn_command(self.build().await?)
    }

    pub fn envs(self) -> LaunchEnvs {
        self.inner
    }
}
