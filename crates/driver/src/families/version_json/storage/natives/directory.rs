use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub(super) struct NativesDirectory<'a> {
    root: &'a Path,
}

impl<'a> NativesDirectory<'a> {
    pub(super) fn new(root: &'a Path) -> Self {
        Self { root }
    }

    pub(super) fn flatten_binaries(self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for (path, file_name) in self.binaries()? {
            let name = file_name.to_string_lossy().into_owned();
            let target = self.root.join(&name);
            if target != path {
                std::fs::copy(&path, &target).map_err(|source| Error::CopyNativeBinary {
                    source_path: path,
                    target_path: target,
                    source,
                })?;
            }
            names.push(name);
        }
        names.sort();
        names.dedup();
        Ok(names)
    }

    fn binaries(self) -> Result<Vec<(PathBuf, OsString)>> {
        let mut pending = vec![self.root.to_path_buf()];
        let mut files = Vec::new();
        while let Some(current) = pending.pop() {
            for entry in std::fs::read_dir(current)? {
                let entry = entry?;
                let path = entry.path();
                if entry.file_type()?.is_dir() {
                    pending.push(path);
                } else if Self::is_binary(&path) {
                    files.push((path, entry.file_name()));
                }
            }
        }
        Ok(files)
    }

    fn is_binary(path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "dll" | "so" | "dylib" | "jnilib"
                )
            })
    }
}
