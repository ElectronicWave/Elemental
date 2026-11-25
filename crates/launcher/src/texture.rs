use std::{collections::HashSet, ffi::OsStr, fmt, io, path::Path, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

pub(crate) const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

pub(crate) fn is_valid_name(name: &str) -> bool {
    !name.trim().is_empty()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvalidTextureHash {
    value: String,
}

impl fmt::Display for InvalidTextureHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid texture hash '{}'", self.value)
    }
}

impl std::error::Error for InvalidTextureHash {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TextureHash([u8; 32]);

impl TextureHash {
    pub(crate) fn of(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let mut value = [0; 32];
        value.copy_from_slice(&digest);
        Self(value)
    }

    pub(crate) fn hex(self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Display for TextureHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.hex())
    }
}

impl FromStr for TextureHash {
    type Err = InvalidTextureHash;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(InvalidTextureHash {
                value: value.to_owned(),
            });
        }

        let mut bytes = [0; 32];
        hex::decode_to_slice(value, &mut bytes).map_err(|_| InvalidTextureHash {
            value: value.to_owned(),
        })?;
        Ok(Self(bytes))
    }
}

impl Serialize for TextureHash {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.hex())
    }
}

impl<'de> Deserialize<'de> for TextureHash {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapeId(TextureHash);

impl fmt::Display for CapeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl CapeId {
    pub(crate) fn from_hash(hash: TextureHash) -> Self {
        Self(hash)
    }

    pub(crate) fn hash(self) -> TextureHash {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SkinId(TextureHash);

impl fmt::Display for SkinId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl SkinId {
    pub(crate) fn from_hash(hash: TextureHash) -> Self {
        Self(hash)
    }

    pub(crate) fn hash(self) -> TextureHash {
        self.0
    }
}

pub(crate) async fn remove_if_present(path: &Path) -> io::Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Removes unreferenced content files.
///
/// Only files laid out as `<root>/<prefix>/<hash>.png` are considered. Other
/// files are left untouched.
pub(crate) async fn gc_content(
    root_path: &Path,
    referenced: &HashSet<TextureHash>,
) -> io::Result<()> {
    let mut prefixes = match tokio::fs::read_dir(root_path).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    while let Some(prefix) = prefixes.next_entry().await? {
        if !prefix.file_type().await?.is_dir() {
            continue;
        }

        let mut files = tokio::fs::read_dir(prefix.path()).await?;
        while let Some(file) = files.next_entry().await? {
            if !file.file_type().await?.is_file() {
                continue;
            }

            gc_file(&file, referenced).await?;
        }
    }

    Ok(())
}

async fn gc_file(file: &tokio::fs::DirEntry, referenced: &HashSet<TextureHash>) -> io::Result<()> {
    let Some(hash) = content_hash(&file.file_name()) else {
        return Ok(());
    };
    if !referenced.contains(&hash) {
        remove_if_present(&file.path()).await?;
    }
    Ok(())
}

fn content_hash(file_name: &OsStr) -> Option<TextureHash> {
    let stem = file_name.to_str()?.strip_suffix(".png")?;
    TextureHash::from_str(stem).ok()
}
