//! Shared filesystem discovery for Java runtime homes.

use std::env::consts::EXE_SUFFIX;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use tokio::fs::{DirEntry, ReadDir};

enum EntryAction {
    RuntimeHome(PathBuf),
    Descend(PathBuf),
    Skip,
}

enum DirectoryAction {
    Inspect,
    RuntimeHome,
    Skip,
}

pub(crate) async fn path_exists(path: &Path) -> bool {
    match tokio::fs::try_exists(path).await {
        Ok(exists) => exists,
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                %error,
                "check Java search path failed"
            );
            false
        }
    }
}

/// Returns whether `path` contains a Java executable at `bin/java`.
pub async fn is_runtime_home(path: &Path) -> bool {
    path_exists(&path.join("bin").join(format!("java{EXE_SUFFIX}"))).await
}

/// Finds Java runtime homes below `root`, including `root` itself.
///
/// `max_depth` is zero-based: `0` checks only `root`, while `1` also checks its
/// immediate children. Missing paths and unreadable directories are skipped.
pub async fn find_runtime_homes(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    find_runtime_homes_inner(root, max_depth, None).await
}

/// Finds Java runtime homes whose immediate child names start with one of
/// `prefixes`.
///
/// The depth and error behavior match [`find_runtime_homes`]. Prefix filtering
/// applies only to direct children of `root`; deeper traversal is unrestricted.
pub async fn find_runtime_homes_with_prefixes(
    root: &Path,
    max_depth: usize,
    prefixes: &[&str],
) -> Vec<PathBuf> {
    find_runtime_homes_inner(root, max_depth, Some(prefixes)).await
}

async fn find_runtime_homes_inner(
    root: &Path,
    max_depth: usize,
    prefixes: Option<&[&str]>,
) -> Vec<PathBuf> {
    let mut homes = Vec::new();
    let mut pending = vec![(root.to_path_buf(), 0usize)];

    while let Some((directory, depth)) = pending.pop() {
        match classify_directory(&directory, depth, max_depth).await {
            DirectoryAction::RuntimeHome => homes.push(directory),
            DirectoryAction::Skip => {}
            DirectoryAction::Inspect => {
                let Some(mut entries) = read_entries(&directory).await else {
                    continue;
                };

                while let Some(entry) = next_entry(&mut entries, &directory).await {
                    match classify_entry(entry, depth, prefixes).await {
                        EntryAction::RuntimeHome(path) => homes.push(path),
                        EntryAction::Descend(path) => pending.push((path, depth + 1)),
                        EntryAction::Skip => {}
                    }
                }
            }
        }
    }

    homes
}

async fn classify_directory(directory: &Path, depth: usize, max_depth: usize) -> DirectoryAction {
    if !path_exists(directory).await {
        return DirectoryAction::Skip;
    }
    if is_runtime_home(directory).await {
        return DirectoryAction::RuntimeHome;
    }
    if depth >= max_depth {
        return DirectoryAction::Skip;
    }

    DirectoryAction::Inspect
}

async fn read_entries(directory: &Path) -> Option<ReadDir> {
    match tokio::fs::read_dir(directory).await {
        Ok(entries) => Some(entries),
        Err(error) => {
            tracing::warn!(
                path = %directory.display(),
                %error,
                "read Java search directory failed"
            );
            None
        }
    }
}

async fn next_entry(entries: &mut ReadDir, directory: &Path) -> Option<DirEntry> {
    match entries.next_entry().await {
        Ok(Some(entry)) => Some(entry),
        Ok(None) => None,
        Err(error) => {
            tracing::warn!(
                path = %directory.display(),
                %error,
                "read Java search directory entry failed"
            );
            None
        }
    }
}

async fn classify_entry(entry: DirEntry, depth: usize, prefixes: Option<&[&str]>) -> EntryAction {
    let path = entry.path();
    let file_type = match entry.file_type().await {
        Ok(file_type) => file_type,
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                %error,
                "read Java search entry type failed"
            );
            return EntryAction::Skip;
        }
    };

    if file_type.is_symlink() {
        return if is_runtime_home(&path).await {
            EntryAction::RuntimeHome(path)
        } else {
            EntryAction::Skip
        };
    }
    if !file_type.is_dir() {
        return EntryAction::Skip;
    }
    if !descend_allowed(depth, prefixes, &entry.file_name()) {
        return EntryAction::Skip;
    }

    EntryAction::Descend(path)
}

fn descend_allowed(depth: usize, prefixes: Option<&[&str]>, file_name: &OsStr) -> bool {
    if depth != 0 {
        return true;
    }
    let Some(prefixes) = prefixes else {
        return true;
    };

    let file_name = file_name.to_string_lossy();
    prefixes.iter().any(|prefix| file_name.starts_with(prefix))
}
