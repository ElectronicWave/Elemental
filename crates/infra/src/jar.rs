use std::{
    ffi::OsStr,
    fs::{File, create_dir_all},
    io::{self, Cursor, Read, Seek},
    path::{Path, PathBuf},
};

use zip::ZipArchive;

use crate::error::Result;

#[cfg(unix)]
use std::{
    ffi::OsString,
    os::unix::{ffi::OsStringExt, fs::PermissionsExt},
    path::Component,
};

#[derive(Debug, Clone)]
pub struct JarFile<P: AsRef<Path>> {
    path: P,
}

#[derive(Debug, Clone, Copy)]
pub struct JarBytes<'a> {
    bytes: &'a [u8],
}

impl<P: AsRef<Path>> JarFile<P> {
    pub fn new(path: P) -> Self {
        Self { path }
    }

    pub fn extract_blocking<S: AsRef<OsStr> + ?Sized>(&self, dest: &S) -> Result<()> {
        let file = File::open(&self.path)?;
        let mut archive = ZipArchive::new(file)?;
        let destination = Path::new(dest);
        extract_entries_blocking(&mut archive, destination, |relative_path| {
            if relative_path.starts_with("META-INF") {
                return None;
            }

            Some(relative_path.to_path_buf())
        })?;

        Ok(())
    }

    pub fn extract_missing_blocking<S, F>(&self, dest: &S, include: F) -> Result<Vec<PathBuf>>
    where
        S: AsRef<OsStr> + ?Sized,
        F: Fn(&Path) -> bool,
    {
        let file = File::open(&self.path)?;
        let mut archive = ZipArchive::new(file)?;
        let destination = Path::new(dest);
        extract_entries_blocking(&mut archive, destination, |relative_path| {
            if !include(relative_path) {
                return None;
            }

            if destination.join(relative_path).symlink_metadata().is_ok() {
                return None;
            }

            Some(relative_path.to_path_buf())
        })
    }

    pub fn by_name_bytes(&self, name: &str) -> Result<Vec<u8>> {
        let file = File::open(&self.path)?;
        read_archive_entry_bytes(file, name)
    }

    pub fn by_name_string(&self, name: &str) -> Result<String> {
        Ok(String::from_utf8(self.by_name_bytes(name)?)?)
    }

    pub fn extract_prefixed_blocking(&self, prefix: &str, dest: &Path) -> Result<Vec<PathBuf>> {
        let file = File::open(&self.path)?;
        let mut archive = ZipArchive::new(file)?;
        let normalized_prefix = PathBuf::from(prefix.trim_matches('/'));

        extract_entries_blocking(&mut archive, dest, |relative_path| {
            let Ok(stripped) = relative_path.strip_prefix(&normalized_prefix) else {
                return None;
            };

            if stripped.as_os_str().is_empty() {
                return None;
            }

            Some(stripped.to_path_buf())
        })
    }

    pub async fn extract_prefixed(&self, prefix: &str, dest: &Path) -> Result<Vec<PathBuf>>
    where
        P: Clone + Send + 'static,
    {
        let jar = self.clone();
        let prefix = prefix.to_owned();
        let dest = dest.to_path_buf();
        tokio::task::spawn_blocking(move || jar.extract_prefixed_blocking(&prefix, &dest)).await?
    }
}

impl<'a> JarBytes<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub fn by_name_bytes(&self, name: &str) -> Result<Vec<u8>> {
        read_archive_entry_bytes(Cursor::new(self.bytes), name)
    }

    pub fn by_name_string(&self, name: &str) -> Result<String> {
        Ok(String::from_utf8(self.by_name_bytes(name)?)?)
    }
}

fn read_archive_entry_bytes<R>(reader: R, name: &str) -> Result<Vec<u8>>
where
    R: Read + Seek,
{
    let mut archive = ZipArchive::new(reader)?;
    let mut data = vec![];
    archive.by_name(name)?.read_to_end(&mut data)?;
    Ok(data)
}

fn extract_entries_blocking<R, F>(
    archive: &mut ZipArchive<R>,
    dest: &Path,
    resolve_output_path: F,
) -> Result<Vec<PathBuf>>
where
    R: Read + Seek,
    F: Fn(&Path) -> Option<PathBuf>,
{
    let mut extracted = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(relative_path) = entry.enclosed_name() else {
            continue;
        };
        let Some(output_relative_path) = resolve_output_path(&relative_path) else {
            continue;
        };
        let output_path = dest.join(&output_relative_path);

        if entry.is_dir() {
            create_dir_all(&output_path)?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            create_dir_all(parent)?;
        }

        if entry.is_symlink() {
            extract_symlink_entry(&mut entry, dest, &output_path)?;
            extracted.push(output_path);
            continue;
        }

        let mut output = File::create(&output_path)?;
        io::copy(&mut entry, &mut output)?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            std::fs::set_permissions(&output_path, std::fs::Permissions::from_mode(mode & 0o777))?;
        }
        extracted.push(output_path);
    }

    Ok(extracted)
}

#[cfg(unix)]
fn extract_symlink_entry<R: Read>(entry: &mut R, dest: &Path, output_path: &Path) -> Result<()> {
    let mut target_bytes = Vec::new();
    entry.read_to_end(&mut target_bytes)?;
    let target = OsString::from_vec(target_bytes);
    let target_path = PathBuf::from(&target);
    let resolved = if target_path.is_absolute() {
        target_path.clone()
    } else {
        output_path.parent().unwrap_or(dest).join(&target_path)
    };
    let resolved = normalize_lexically(&resolved);
    let destination = normalize_lexically(dest);
    if !resolved.starts_with(&destination) {
        return Err(crate::error::Error::ZipSymlinkEscapesDestination {
            entry: output_path.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
        });
    }

    if output_path.symlink_metadata().is_err() {
        std::os::unix::fs::symlink(&target, output_path)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn extract_symlink_entry<R: Read>(_entry: &mut R, _dest: &Path, _output_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    normalized
}
