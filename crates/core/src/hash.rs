use std::{fs::read, io::Result, path::Path};
use sha1::{Digest, Sha1};

#[inline]
pub fn sha1<D: AsRef<[u8]>>(data: D) -> String {
    hex::encode(Sha1::digest(data))
}

#[inline]
pub fn file_sha1<P: AsRef<Path>>(path: P) -> Result<String> {
    Ok(sha1(read(path)?))
}

#[inline]
pub async fn async_file_sha1<P: AsRef<Path>>(path: P) -> Result<String> {
    Ok(sha1(tokio::fs::read(path).await?))
}
