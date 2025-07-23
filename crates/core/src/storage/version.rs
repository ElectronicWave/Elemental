use std::fs::{File, create_dir_all};
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf, absolute};

use tokio::process::{Child, Command};

use crate::consts::PLATFORM_NATIVES_DIR_NAME;
use crate::error::unification::UnifiedResult;
use crate::model::launchenvs::LaunchEnvs;
use crate::model::mojang::VersionData;
use crate::storage::game::GameStorage;

pub struct VersionStorage {
    pub root: String,
    pub name: String,
}

impl VersionStorage {
    pub fn new_unchecked(root: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            name: name.into(),
        }
    }

    pub fn new_abs_unchecked(root: impl Into<String>, name: impl Into<String>) -> Result<Self> {
        let root = absolute(root.into())?;
        Ok(Self {
            root: root.to_string_lossy().to_string(),
            name: name.into(),
        })
    }

    pub fn new(root: impl Into<String>, name: impl Into<String>) -> Result<Self> {
        let root = absolute(root.into())?;
        let name = name.into();

        if root
            .file_name()
            .map(|r| r.to_string_lossy().to_string() != name)
            .unwrap_or(true)
        {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Version `{name}` has a different name with it's root."),
            ));
        }

        Ok(Self {
            root: root.to_string_lossy().to_string(),
            name,
        })
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        Path::new(&self.root).join(path)
    }

    pub fn pistonmeta(&self) -> Result<VersionData> {
        serde_json::from_reader(File::open(self.join(format!("{}.json", self.name)))?).to_stdio()
    }

    pub fn get_ensure_natives_path(&self) -> Result<PathBuf> {
        let path = self.join(PLATFORM_NATIVES_DIR_NAME);
        create_dir_all(&path)?;
        Ok(path)
    }

    pub fn validate_version_data(&self) {
        //TODO validate version
    }

    pub fn get_ensure_subpath<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        let subpath = self.join(path);
        create_dir_all(&subpath)?;
        Ok(subpath)
    }

    pub fn launch(
        &self,
        storage: &GameStorage,
        executable: impl Into<String>,
        extra_args: impl IntoIterator<Item = String>,
    ) -> Result<Child> {
        let envs = LaunchEnvs::offline_player(
            "Test".to_owned(), //TODO
            storage.root.clone(),
            self.root.clone(),
            &self.pistonmeta()?,
        )?;
        let pistonmeta = self.pistonmeta()?;
        let mut args = vec![];
        args.extend(extra_args);
        args.extend(envs.apply_launchenvs(pistonmeta.arguments.get_jvm_arguments())?);
        args.push(pistonmeta.main_class.clone());
        args.extend(envs.apply_launchenvs(pistonmeta.arguments.get_game_arguments())?);
        //TODO Customize Output
        Ok(Command::new(executable.into()).args(args).spawn()?)
    }
}

pub enum VersionLaunchMode {
    Offline,
}
