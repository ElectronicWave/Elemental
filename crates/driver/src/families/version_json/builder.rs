use std::{
    collections::{HashMap, HashSet},
    env::current_dir,
    path::PathBuf,
};

use anyhow::{Context, Result, bail};
use elemental_core::{
    auth::{authorizer::Authorizer, credential::UserCredential},
    launcher::command::LaunchCommand,
    runtime::distribution::Distribution,
    storage::{Storage, layout::Layoutable},
};
use elemental_schema::mojang::piston::PistonMetaData;

use super::{
    classpath::{classpath_separator, join_classpath},
    extensions::{PistonMetaDataExt, PistonMetaLibrariesExt},
    launch::parse_argument_string,
    layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    resource::{VersionJsonInstanceResource, VersionJsonRootResource},
    rules::VersionJsonRuleContext,
    storage::VersionJsonVersionStorageExt,
    variables::{LauncherVariables, UserType},
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
        let version_name = self.version.name().context("get version name failed")?;
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

    fn build_command_arguments(
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
        let mut jvm = Vec::new();

        // Legacy metadata may already be normalized into `arguments.game`, while still needing
        // the historical JVM bootstrap arguments such as `-cp` and `-Djava.library.path`.
        if raw_jvm_arguments.is_empty() {
            jvm.extend(self.inner.apply(legacy_jvm_arguments())?);
        }

        if let Some(logging_arguments) = self.logging_arguments(metadata)? {
            jvm.extend(logging_arguments);
        }

        jvm.extend(self.inner.apply(raw_jvm_arguments)?);
        jvm.extend(self.inner.apply(self.extra_jvm_arguments.clone())?);

        Ok(jvm)
    }

    fn logging_arguments(&self, metadata: &PistonMetaData) -> Result<Option<Vec<String>>> {
        let Some(logging) = &metadata.logging else {
            return Ok(None);
        };
        let Some(client) = &logging.client else {
            return Ok(None);
        };

        let log_config_path =
            self.version
                .parent
                .try_get_resource(VersionJsonRootResource::AssetLogConfigs(Some(
                    client.file.id.clone(),
                )))?;
        let log_config_path = resolve_absolute_path(log_config_path)?;

        if !log_config_path.exists() {
            bail!(
                "logging config not found: {}",
                log_config_path.to_string_lossy()
            );
        }

        Ok(Some(self.inner.apply_with(
            vec![client.argument.clone()],
            &HashMap::from([(
                "path".to_owned(),
                log_config_path.to_string_lossy().to_string(),
            )]),
        )?))
    }

    fn build_classpath(
        &self,
        metadata: &PistonMetaData,
        rule_context: &VersionJsonRuleContext,
        paths: &LaunchPaths,
        module_path_entries: &HashSet<String>,
    ) -> Result<String> {
        let mut classpath = metadata
            .libraries
            .iter()
            .map(|library| -> Result<Option<String>> {
                if !library.is_allowed(rule_context) {
                    return Ok(None);
                }

                let Some(artifact) = library.downloads.artifact.as_ref() else {
                    return Ok(None);
                };
                if artifact.path.contains("natives") {
                    return Ok(None);
                }

                let path =
                    self.version
                        .parent
                        .try_get_resource(VersionJsonRootResource::Libraries(Some(
                            PathBuf::from(artifact.path.as_str()),
                        )))?;
                let path = resolve_absolute_path(path)?.to_string_lossy().to_string();
                if module_path_entries.contains(&normalize_path_string(path.as_str())) {
                    return Ok(None);
                }

                Ok(Some(path))
            })
            .collect::<Result<Vec<Option<String>>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<String>>();

        classpath.push(paths.version_jar.to_string_lossy().to_string());

        Ok(join_classpath(classpath))
    }

    fn collect_module_path_entries(&self, jvm_arguments: &[String]) -> Result<HashSet<String>> {
        let mut entries = HashSet::new();
        let mut index = 0usize;

        while index < jvm_arguments.len() {
            let argument = &jvm_arguments[index];
            if let Some(raw_value) = argument
                .strip_prefix("--module-path=")
                .or_else(|| argument.strip_prefix("-p="))
            {
                entries.extend(self.resolve_module_path_entries(raw_value)?);
                index += 1;
                continue;
            }

            if argument == "-p" || argument == "--module-path" {
                let Some(raw_value) = jvm_arguments.get(index + 1) else {
                    bail!("module path argument is missing its value");
                };
                entries.extend(self.resolve_module_path_entries(raw_value)?);
                index += 2;
                continue;
            }

            index += 1;
        }

        Ok(entries)
    }

    fn resolve_module_path_entries(&self, raw_value: &str) -> Result<Vec<String>> {
        let resolved = self
            .inner
            .apply(vec![raw_value.to_owned()])?
            .into_iter()
            .next()
            .context("resolve module path argument failed")?;

        Ok(resolved
            .split(classpath_separator())
            .filter(|value| !value.is_empty())
            .map(normalize_path_string)
            .collect())
    }
}

#[derive(Debug, Clone)]
struct LaunchPaths {
    version_root: PathBuf,
    version_jar: PathBuf,
    assets_root: PathBuf,
    libraries_root: PathBuf,
    natives_directory: PathBuf,
}

impl LaunchPaths {
    fn resolve<L, VL>(version: &Storage<VL, Storage<L>>) -> Result<Self>
    where
        L: VersionJsonRootLayout,
        VL: VersionJsonInstanceLayout,
    {
        Ok(Self {
            version_root: resolve_absolute_path(version.path.clone())?,
            version_jar: resolve_absolute_path(
                version.try_get_resource(VersionJsonInstanceResource::Jar)?,
            )?,
            assets_root: resolve_absolute_path(
                version
                    .parent
                    .try_get_resource(VersionJsonRootResource::Assets)?,
            )?,
            libraries_root: resolve_absolute_path(
                version
                    .parent
                    .try_get_resource(VersionJsonRootResource::Libraries(None))?,
            )?,
            natives_directory: resolve_absolute_path(
                version.try_get_resource(VersionJsonInstanceResource::Natives)?,
            )?,
        })
    }

    fn ensure_version_jar_exists(&self) -> Result<()> {
        if self.version_jar.exists() {
            return Ok(());
        }

        bail!(
            "version jar not found: {}",
            self.version_jar.to_string_lossy()
        )
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

fn normalize_path_string(path: &str) -> String {
    PathBuf::from(path)
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .to_string()
}

fn resolve_absolute_path(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }

    Ok(current_dir()?.join(path))
}
