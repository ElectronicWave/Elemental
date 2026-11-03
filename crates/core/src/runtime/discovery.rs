//! Shared filesystem discovery for Java runtime homes.

use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};

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
        if !path_exists(&directory).await {
            continue;
        }
        if is_runtime_home(&directory).await {
            homes.push(directory);
            continue;
        }
        if depth >= max_depth {
            continue;
        }

        let mut entries = match tokio::fs::read_dir(&directory).await {
            Ok(entries) => entries,
            Err(error) => {
                tracing::warn!(
                    path = %directory.display(),
                    %error,
                    "read Java search directory failed"
                );
                continue;
            }
        };

        loop {
            let entry = match entries.next_entry().await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(error) => {
                    tracing::warn!(
                        path = %directory.display(),
                        %error,
                        "read Java search directory entry failed"
                    );
                    break;
                }
            };

            let file_type = match entry.file_type().await {
                Ok(file_type) => file_type,
                Err(error) => {
                    tracing::warn!(
                        path = %entry.path().display(),
                        %error,
                        "read Java search entry type failed"
                    );
                    continue;
                }
            };
            if file_type.is_symlink() {
                let path = entry.path();
                if is_runtime_home(&path).await {
                    homes.push(path);
                }
                continue;
            }
            if !file_type.is_dir() {
                continue;
            }

            if depth == 0
                && let Some(prefixes) = prefixes
            {
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                if !prefixes.iter().any(|prefix| file_name.starts_with(prefix)) {
                    continue;
                }
            }

            pending.push((entry.path(), depth + 1));
        }
    }

    homes
}
