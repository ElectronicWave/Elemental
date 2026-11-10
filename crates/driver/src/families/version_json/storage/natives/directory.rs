use std::path::{Path, PathBuf};

use crate::error::Result;

#[derive(Debug, Clone, Copy)]
pub(super) struct NativesDirectory<'a> {
    root: &'a Path,
}

impl<'a> NativesDirectory<'a> {
    pub(super) fn new(root: &'a Path) -> Self {
        Self { root }
    }

    pub(super) fn binary_paths(self) -> Result<Vec<String>> {
        let mut paths = Vec::new();
        let mut pending = vec![(self.root.to_path_buf(), PathBuf::new())];
        while let Some((current, relative)) = pending.pop() {
            for entry in std::fs::read_dir(current)? {
                let entry = entry?;
                let path = entry.path();
                let relative_path = relative.join(entry.file_name());
                if entry.file_type()?.is_dir() {
                    pending.push((path, relative_path));
                    continue;
                }
                if !Self::is_binary(&path) {
                    continue;
                }
                paths.push(relative_path_key(&relative_path));
            }
        }
        paths.sort();
        paths.dedup();
        Ok(paths)
    }

    pub(super) fn is_binary(path: &Path) -> bool {
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

pub(super) fn relative_path_key(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
