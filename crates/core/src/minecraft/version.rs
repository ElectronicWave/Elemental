use std::{fmt::Display, str::FromStr};

use crate::version::Version;

mod error;
mod kind;
mod parse;

pub use error::MinecraftVersionParseError;
pub use kind::{
    AlphaBetaVersion, LegacyReleaseVersion, LegacySnapshotVersion, MinecraftVersionKey,
    MinecraftVersionKind, VersionStage, YearReleaseVersion, YearSnapshotVersion,
};

use parse::{normalize_year, parse_alpha_beta, parse_legacy_snapshot, parse_release};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MinecraftVersionTag;

pub type MinecraftVersionId = Version<MinecraftVersionTag>;

/// A parsed Minecraft version.
///
/// The identifier keeps the exact upstream string, while [`MinecraftVersion::key`]
/// provides a stable semantic ordering across legacy and year-based version schemes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MinecraftVersion {
    id: MinecraftVersionId,
    kind: MinecraftVersionKind,
    key: MinecraftVersionKey,
}

impl MinecraftVersion {
    pub fn parse(raw: &str) -> Result<Self, MinecraftVersionParseError> {
        ensure_not_empty(raw)?;
        parse_known_format(raw)
    }

    pub fn parse_id(id: &MinecraftVersionId) -> Result<Self, MinecraftVersionParseError> {
        Self::parse(id.as_str())
    }

    pub fn legacy_release(major: u32, minor: u32, patch: u32) -> Self {
        let raw = format!("{major}.{minor}.{patch}");
        let kind = MinecraftVersionKind::LegacyRelease(LegacyReleaseVersion {
            major,
            minor,
            patch,
            stage: VersionStage::Release,
            stage_number: 0,
        });
        Self::from_kind(&raw, kind)
    }

    pub fn year_release(year: u32, release: u32, patch: u32) -> Self {
        let full_year = normalize_year(year);
        let raw = canonical_year_id(full_year, release, patch);
        let kind = MinecraftVersionKind::YearRelease(YearReleaseVersion {
            year: full_year,
            release,
            patch,
            stage: VersionStage::Release,
            stage_number: 0,
        });
        Self::from_kind(&raw, kind)
    }

    pub fn id(&self) -> &MinecraftVersionId {
        &self.id
    }

    pub fn into_id(self) -> MinecraftVersionId {
        self.id
    }

    pub fn raw(&self) -> &str {
        self.id.as_str()
    }

    pub fn kind(&self) -> &MinecraftVersionKind {
        &self.kind
    }

    pub fn key(&self) -> MinecraftVersionKey {
        self.key
    }

    fn from_kind(raw: &str, kind: MinecraftVersionKind) -> Self {
        Self {
            id: MinecraftVersionId::from(raw),
            key: kind.into(),
            kind,
        }
    }
}

fn ensure_not_empty(raw: &str) -> Result<(), MinecraftVersionParseError> {
    if raw.is_empty() {
        Err(MinecraftVersionParseError::Empty)
    } else {
        Ok(())
    }
}

fn parse_known_format(raw: &str) -> Result<MinecraftVersion, MinecraftVersionParseError> {
    if let Some(version) = parse_alpha_beta(raw)? {
        return Ok(version);
    }

    match parse_legacy_snapshot(raw)? {
        Some(version) => Ok(version),
        None => parse_release(raw),
    }
}

fn canonical_year_id(year: u32, release: u32, patch: u32) -> String {
    let display_year = if (2000..=2099).contains(&year) {
        year - 2000
    } else {
        year
    };

    if patch == 0 {
        format!("{display_year}.{release}")
    } else {
        format!("{display_year}.{release}.{patch}")
    }
}

impl Display for MinecraftVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.raw())
    }
}

impl FromStr for MinecraftVersion {
    type Err = MinecraftVersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for MinecraftVersion {
    type Error = MinecraftVersionParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for MinecraftVersion {
    type Error = MinecraftVersionParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value.as_str())
    }
}

impl TryFrom<&MinecraftVersionId> for MinecraftVersion {
    type Error = MinecraftVersionParseError;

    fn try_from(value: &MinecraftVersionId) -> Result<Self, Self::Error> {
        Self::parse_id(value)
    }
}

impl TryFrom<MinecraftVersionId> for MinecraftVersion {
    type Error = MinecraftVersionParseError;

    fn try_from(value: MinecraftVersionId) -> Result<Self, Self::Error> {
        Self::parse_id(&value)
    }
}

#[cfg(test)]
mod tests;
