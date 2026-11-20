use std::{collections::HashSet, fmt, io, path::Path, str::FromStr};

use elemental_shared::scope::Scope;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

pub(crate) const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(Error::TextureNameEmpty);
    }
    Ok(())
}

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
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        if value.len() != 64 {
            return Err(Error::InvalidTextureHash {
                value: value.to_owned(),
            });
        }

        let mut bytes = [0; 32];
        hex::decode_to_slice(value, &mut bytes).map_err(|_| Error::InvalidTextureHash {
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

async fn remove_if_present(path: &Path) -> Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Removes unreferenced content files.
///
/// Only files laid out as `<root>/<prefix>/<hash>.png` are considered. Other
/// files are left untouched.
pub(crate) async fn gc_content(
    scope: &Scope,
    root: &str,
    referenced: &HashSet<TextureHash>,
) -> Result<()> {
    let root_path = scope.get_full_path(root, None).await?;
    let mut prefixes = match tokio::fs::read_dir(&root_path).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
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

            let path = file.path();
            let Some(name) = file.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some(stem) = name.strip_suffix(".png") else {
                continue;
            };
            let Ok(hash) = TextureHash::from_str(stem) else {
                continue;
            };
            if !referenced.contains(&hash) {
                remove_if_present(&path).await?;
            }
        }
    }

    Ok(())
}
