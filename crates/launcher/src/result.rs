use std::{
    fs,
    path::{Path, PathBuf},
};

use elemental_addon::{InstalledAddon, InstalledAddonRecord, PatchMode, PistonMetaData};
use elemental_core::{
    auth::credential::Credential,
    launcher::command::LaunchCommand,
    minecraft::MinecraftVersionId,
    runtime::{RuntimeResolver, distribution::Distribution},
    storage::layout::Layoutable,
};
use elemental_driver::{
    driver::{DriverDescriptor, InstalledDriver},
    families::version_json::{
        InstanceStorage, PreparedVersionJsonInstance, VersionJsonInstanceResource,
        builder::VersionJsonLaunchBuilder, variables::LauncherVariables,
    },
    launch::resolve_prepared_version_runtime,
};

use crate::{
    error::{Error, Result},
    request::{LaunchOptions, RuntimeOptions},
};

pub struct LaunchedInstance {
    pub runtime: Distribution,
    pub child: tokio::process::Child,
}

impl LaunchedInstance {
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }

    pub async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }

    pub async fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill().await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveInstanceOutcome {
    Removed,
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub instance_name: String,
    pub instance_root: PathBuf,
    pub driver: InstalledDriver,
    pub addons: Vec<InstalledAddon>,
}

#[derive(Debug)]
pub struct InspectReport {
    pub instances: Vec<Instance>,
    pub failures: Vec<InstanceInspectionFailure>,
}

#[derive(Debug, thiserror::Error)]
#[error(
    "inspect instance '{instance_name}' at '{}' failed: {error}",
    path.display()
)]
pub struct InstanceInspectionFailure {
    pub instance_name: String,
    pub path: PathBuf,
    #[source]
    pub error: Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedInstanceMetadata {
    pub id: String,
    pub main_class: String,
    pub inherits_from: Option<String>,
    pub asset_index: String,
    pub release_type: String,
    pub required_java_major_version: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedInstanceInfo {
    pub metadata: PreparedInstanceMetadata,
    pub version_root: PathBuf,
    pub metadata_path: PathBuf,
    pub version_jar: PathBuf,
    pub natives_directory: PathBuf,
    pub natives_files: Vec<PathBuf>,
    pub classpath: Vec<PathBuf>,
}

pub struct PreparedInstance {
    pub(crate) driver: DriverDescriptor,
    pub(crate) game_version: MinecraftVersionId,
    pub(crate) launch_version: PreparedVersionJsonInstance,
    pub(crate) patch_mode: PatchMode,
    pub(crate) addons: Vec<InstalledAddonRecord>,
}

impl PreparedInstance {
    pub(crate) fn new(
        driver: DriverDescriptor,
        game_version: MinecraftVersionId,
        launch_version: PreparedVersionJsonInstance,
        patch_mode: PatchMode,
        addons: Vec<InstalledAddonRecord>,
    ) -> Self {
        Self {
            driver,
            game_version,
            launch_version,
            patch_mode,
            addons,
        }
    }

    pub fn driver(&self) -> &DriverDescriptor {
        &self.driver
    }

    pub fn game_version(&self) -> &MinecraftVersionId {
        &self.game_version
    }

    pub fn patch_mode(&self) -> PatchMode {
        self.patch_mode
    }

    pub fn addons(&self) -> Vec<InstalledAddon> {
        self.addons
            .iter()
            .map(|record| record.to_installed(self.patch_mode))
            .collect()
    }

    pub fn instance_root(&self) -> &Path {
        self.launch_version.instance_root()
    }

    pub fn version(&self) -> &InstanceStorage {
        &self.launch_version.version
    }

    pub(crate) async fn resolve_runtime(
        &self,
        runtime_options: &RuntimeOptions,
        runtime_resolver: &RuntimeResolver,
    ) -> Result<Distribution> {
        Ok(resolve_prepared_version_runtime(
            &self.launch_version,
            runtime_options,
            runtime_resolver,
        )
        .await?)
    }

    pub(crate) fn build_launch_command(
        &self,
        credential: &Credential,
        runtime: Distribution,
        options: &LaunchOptions,
        metadata_override: Option<PistonMetaData>,
    ) -> Result<LaunchCommand> {
        let mut variables = LauncherVariables::default();

        if let Some(client_id) = &options.client_id {
            variables.clientid = client_id.clone();
        }

        if let Some(resolution) = &options.resolution {
            variables.resolution_width = resolution.width.clone();
            variables.resolution_height = resolution.height.clone();
        }

        if let (Some(name), Some(version)) = (&options.launcher_name, &options.launcher_version) {
            variables.launcher_name = name.clone();
            variables.launcher_version = version.clone();
        }

        if let Some(quick_play) = &options.quick_play {
            variables.quick_play_path = quick_play.path.clone();
            variables.quick_play_singleplayer = quick_play.singleplayer.clone();
            variables.quick_play_multiplayer = quick_play.multiplayer.clone();
            variables.quick_play_realms = quick_play.realms.clone();
        }

        let builder = VersionJsonLaunchBuilder::new(
            credential.clone(),
            runtime,
            self.launch_version.version.clone(),
            variables,
            options.extra_jvm_arguments.clone(),
            options.extra_game_arguments.clone(),
        )
        .with_metadata(metadata_override.unwrap_or_else(|| self.launch_version.metadata.clone()));
        Ok(builder.build_command()?)
    }

    pub fn info(&self) -> Result<PreparedInstanceInfo> {
        let version = &self.launch_version.version;
        let metadata_path = version.try_get_resource(VersionJsonInstanceResource::Metadata)?;
        let version_jar = version.try_get_resource(VersionJsonInstanceResource::Jar)?;
        let natives_directory = version.try_get_resource(VersionJsonInstanceResource::Natives)?;
        let mut natives_files = Vec::new();
        collect_files(&natives_directory, &mut natives_files)?;
        natives_files.sort();

        Ok(PreparedInstanceInfo {
            metadata: PreparedInstanceMetadata {
                id: self.launch_version.metadata.id.clone(),
                main_class: self.launch_version.metadata.main_class.clone(),
                inherits_from: self.launch_version.metadata.inherits_from.clone(),
                asset_index: self.launch_version.metadata.asset_index.id.clone(),
                release_type: self.launch_version.metadata.release_type.clone(),
                required_java_major_version: self.launch_version.required_java_major_version(),
            },
            version_root: version.path.clone(),
            metadata_path,
            version_jar,
            natives_directory,
            natives_files,
            classpath: self.launch_version.classpath()?,
        })
    }
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }

    Ok(())
}
