use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf, absolute};

use crate::error::unification::UnifiedResult;
use crate::model::mojang::PistonMetaData;

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

    pub fn pistonmeta(&self) -> Result<PistonMetaData> {
        serde_json::from_reader(File::open(self.join(format!("{}.json", self.name)))?).to_stdio()
    }
}
