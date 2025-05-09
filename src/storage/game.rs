use std::fs::create_dir_all;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

use crate::model::mojang::PistonMetaLibrariesDownloadsArtifact;

pub struct GameStorage {
    root: String, // ..../.minecraft
}

impl GameStorage {
    pub fn new(root: impl Into<String>) -> Self {
        Self { root: root.into() }
    }

    pub fn new_ensure_dir(root: impl Into<String>) -> Result<Self> {
        let root = root.into();
        if let Err(err) = create_dir_all(&root) {
            Err(err)
        } else {
            Ok(Self { root })
        }
    }

    pub fn get_ensure_object_path(&self, hash: String) -> Result<String> {
        let parent = self
            .join("assets")
            .join("objects")
            .join(hash.get(0..2).unwrap());

        if let Err(err) = create_dir_all(parent.clone()) {
            Err(err)
        } else {
            Ok(parent.join(hash).to_string_lossy().to_string())
        }
    }

    pub fn get_object_indexes_path(&self, version: String) -> String {
        self.join("assets")
            .join("indexes") // ensure it is created
            .join(version)
            .to_string_lossy()
            .to_string()
    }

    pub fn get_natives_path(&self) -> String {
        todo!()
    }

    pub fn get_ensure_library_path(
        &self,
        library: PistonMetaLibrariesDownloadsArtifact,
    ) -> Result<String> {
        let path = self.join("libraries").join(&library.path);
        let path_parent = path.parent();

        if let None = path_parent {
            return Err(Error::new(ErrorKind::Other, "No such directory"));
        }

        if let Err(err) = create_dir_all(path_parent.unwrap()) {
            Err(err)
        } else {
            Ok(path.to_string_lossy().to_string())
        }
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        Path::new(&self.root).join(path)
    }

    pub fn download_version(&self) {
        todo!()
    }
    pub fn download_objects(&self) {
        todo!()
    }
    pub fn download_pistonmeta_all(&self) {
        todo!()
    }
    pub fn validate_version() {
        todo!()
    }
}
