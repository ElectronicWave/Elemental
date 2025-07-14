use std::{
    ffi::OsStr,
    fs::{File, create_dir_all},
    io::{self, Read, Result},
    path::Path,
};
use zip::ZipArchive;

use crate::error::unification::UnifiedResult;

// Support `Deflate/Stored` Now
#[derive(Debug, Clone)]
pub struct JarFile<P: AsRef<Path>> {
    path: P,
}

impl<P: AsRef<Path>> JarFile<P> {
    pub fn new(path: P) -> Self {
        Self { path }
    }

    //TODO May need extract more efficient
    pub fn extract_blocking<S: AsRef<OsStr> + ?Sized>(&self, dest: &S) -> Result<()> {
        let file = File::open(&self.path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;

            let outpath = match file.enclosed_name() {
                Some(path) => path,
                None => continue,
            };

            // Ignore `META-INF`
            if outpath.starts_with("META-INF") {
                continue;
            }

            //? Ignore `.sha1`

            let outpath = Path::new(dest).join(outpath);

            if file.is_dir() {
                create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        create_dir_all(p)?;
                    }
                }
                let mut outfile = File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }

            //TODO sha1 check
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
        String::from_utf8(self.by_name_bytes(name)?).to_stdio()
    }
}

#[test]
fn test_extract() {
    JarFile::new("lwjgl-tinyfd-3.2.2-natives-windows.jar")
        .extract_blocking("output")
        .unwrap();
}

#[test]
fn test_file() {
    println!(
        "{}",
        JarFile::new("Botania-1.16.5-420.3.jar")
            .by_name_string("META-INF/mods.toml")
            .unwrap()
    );
}
