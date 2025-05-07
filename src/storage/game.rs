use std::fs::create_dir_all;
use std::io::Result;
use std::path::{Path, PathBuf};

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

    pub fn get_object_path(&self, hash: String) -> String {
        self.join("assets")
            .join("objects")
            .join(hash.get(0..2).unwrap())
            .join(hash)
            .to_string_lossy()
            .to_string()
    }

    pub fn get_natives_path() -> String {
        todo!()
    }

    pub fn get_library_path(&self) -> String {
        todo!()
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        Path::new(&self.root).join(path)
    }
}
