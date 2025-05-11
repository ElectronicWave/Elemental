use sha1_smol::Sha1;
use std::{fs::read, io::Result};

#[inline]
pub fn sha1<D: AsRef<[u8]>>(data: D) -> String {
    Sha1::from(data).digest().to_string()
}

#[inline]
pub fn file_sha1(path: String) -> Result<String> {
    Ok(sha1(read(path)?))
}

#[inline]
pub fn validate_file_sha1(path: String, hash: String) -> Result<bool> {
    Ok(hash == file_sha1(path)?)
}
