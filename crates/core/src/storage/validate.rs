use sha1_smol::Sha1;
use std::{fs::read, io::Result, path::Path};

#[inline]
pub fn sha1<D: AsRef<[u8]>>(data: D) -> String {
    Sha1::from(data).digest().to_string()
}

#[inline]
pub fn file_sha1<P: AsRef<Path>>(path: P) -> Result<String> {
    Ok(sha1(read(path)?))
}
