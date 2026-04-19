use anyhow::Result;
use std::{
    ffi::OsStr,
    fs::{File, create_dir_all},
    io::{self, Read},
    path::Path,
};
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub struct JarFile<P: AsRef<Path>> {
    path: P,
}

impl<P: AsRef<Path>> JarFile<P> {
    pub fn new(path: P) -> Self {
        Self { path }
    }

    pub fn extract_blocking<S: AsRef<OsStr> + ?Sized>(&self, dest: &S) -> Result<()> {
        let file = File::open(&self.path)?;
        let mut archive = ZipArchive::new(file)?;

        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let relative_path = match entry.enclosed_name() {
                Some(path) => path,
                None => continue,
            };

            if relative_path.starts_with("META-INF") {
                continue;
            }

            let output_path = Path::new(dest).join(relative_path);
            if entry.is_dir() {
                create_dir_all(&output_path)?;
                continue;
            }

            if let Some(parent) = output_path.parent() {
                create_dir_all(parent)?;
            }

            let mut output = File::create(output_path)?;
            io::copy(&mut entry, &mut output)?;
        }

        Ok(())
    }

    pub fn by_name_bytes(&self, name: &str) -> Result<Vec<u8>> {
        let file = File::open(&self.path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut data = vec![];
        archive.by_name(name)?.read_to_end(&mut data)?;
        Ok(data)
    }

    pub fn by_name_string(&self, name: &str) -> Result<String> {
        Ok(String::from_utf8(self.by_name_bytes(name)?)?)
    }
}
