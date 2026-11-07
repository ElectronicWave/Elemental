use std::{fmt::Display, str::FromStr};

use crate::version::Version;

mod error;
mod kind;

pub use error::MinecraftVersionParseError;
pub use kind::{
    AlphaBetaVersion, LegacyReleaseVersion, LegacySnapshotVersion, MinecraftVersionKey,
    MinecraftVersionKind, VersionStage, YearReleaseVersion, YearSnapshotVersion,
};

use kind::VersionEra;

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
        if raw.is_empty() {
            return Err(MinecraftVersionParseError::Empty);
        }

        if let Some(version) = parse_alpha_beta(raw)? {
            return Ok(version);
        }
        if let Some(version) = parse_legacy_snapshot(raw)? {
            return Ok(version);
        }

        parse_release(raw)
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
        let key = MinecraftVersionKey {
            era: VersionEra::Legacy,
            major,
            minor,
            patch,
            stage: VersionStage::Release,
            stage_number: 0,
        };

        Self::from_parts(&raw, kind, key)
    }

    pub fn year_release(year: u32, release: u32, patch: u32) -> Self {
        let full_year = normalize_year(year);
        let display_year = if (2000..=2099).contains(&full_year) {
            full_year - 2000
        } else {
            full_year
        };
        let raw = if patch == 0 {
            format!("{display_year}.{release}")
        } else {
            format!("{display_year}.{release}.{patch}")
        };
        let kind = MinecraftVersionKind::YearRelease(YearReleaseVersion {
            year: full_year,
            release,
            patch,
            stage: VersionStage::Release,
            stage_number: 0,
        });
        let key = MinecraftVersionKey {
            era: VersionEra::Year,
            major: full_year,
            minor: release,
            patch,
            stage: VersionStage::Release,
            stage_number: 0,
        };

        Self::from_parts(&raw, kind, key)
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

    fn from_parts(raw: &str, kind: MinecraftVersionKind, key: MinecraftVersionKey) -> Self {
        Self {
            id: MinecraftVersionId::from(raw),
            kind,
            key,
        }
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

fn parse_alpha_beta(raw: &str) -> Result<Option<MinecraftVersion>, MinecraftVersionParseError> {
    let Some((era, stage)) = raw.chars().next().and_then(|character| match character {
        'a' => Some((VersionEra::Alpha, VersionStage::Alpha)),
        'b' => Some((VersionEra::Beta, VersionStage::Beta)),
        _ => None,
    }) else {
        return Ok(None);
    };
    let numeric = &raw[1..];
    if !numeric.starts_with(|character: char| character.is_ascii_digit()) {
        return Ok(None);
    }

    let numbers = parse_numeric_components(raw, numeric)?;
    let (major, minor, patch) = match numbers.as_slice() {
        [major, minor] => (*major, *minor, 0),
        [major, minor, patch] => (*major, *minor, *patch),
        _ => {
            return Err(MinecraftVersionParseError::InvalidFormat {
                raw: raw.to_owned(),
                reason: "alpha/beta version must have two or three numeric components",
            });
        }
    };
    let kind = MinecraftVersionKind::AlphaBeta(AlphaBetaVersion {
        stage,
        major,
        minor,
        patch,
    });
    let key = MinecraftVersionKey {
        era,
        major,
        minor,
        patch,
        stage,
        stage_number: 0,
    };

    Ok(Some(MinecraftVersion::from_parts(raw, kind, key)))
}

fn parse_legacy_snapshot(
    raw: &str,
) -> Result<Option<MinecraftVersion>, MinecraftVersionParseError> {
    let Some((year_part, rest)) = raw.split_once('w') else {
        return Ok(None);
    };
    if year_part.len() != 2
        || !year_part
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Ok(None);
    }

    let digit_count = rest
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digit_count == 0 {
        return Ok(None);
    }

    let week_part = &rest[..digit_count];
    let suffix_part = &rest[digit_count..];
    if suffix_part.chars().count() > 1
        || !suffix_part
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        return Ok(None);
    }

    let year = parse_component(raw, year_part)?;
    let full_year = 2000 + year;
    let week = parse_component(raw, week_part)?;
    let suffix = suffix_part
        .chars()
        .next()
        .map(|character| character.to_ascii_lowercase());
    let kind = MinecraftVersionKind::LegacySnapshot(LegacySnapshotVersion {
        year: full_year,
        week,
        suffix,
    });
    let key = MinecraftVersionKey {
        era: VersionEra::Legacy,
        major: full_year,
        minor: week,
        patch: 0,
        stage: VersionStage::Snapshot,
        stage_number: snapshot_suffix_rank(suffix),
    };

    Ok(Some(MinecraftVersion::from_parts(raw, kind, key)))
}

fn parse_release(raw: &str) -> Result<MinecraftVersion, MinecraftVersionParseError> {
    let (base, stage_part) = match raw.split_once('-') {
        Some((base, stage)) => (base, Some(stage)),
        None => (raw, None),
    };
    let numbers = parse_numeric_components(raw, base)?;
    if !(2..=3).contains(&numbers.len()) {
        return Err(MinecraftVersionParseError::InvalidFormat {
            raw: raw.to_owned(),
            reason: "version must have two or three numeric components",
        });
    }

    let (stage, stage_number) = match stage_part {
        Some(stage) => parse_stage(raw, stage)?,
        None => (VersionStage::Release, 0),
    };
    let first = numbers[0];

    if first == 1 {
        if stage == VersionStage::Snapshot {
            return Err(MinecraftVersionParseError::InvalidFormat {
                raw: raw.to_owned(),
                reason: "legacy releases cannot use a snapshot stage",
            });
        }

        let minor = numbers[1];
        let patch = numbers.get(2).copied().unwrap_or(0);
        let kind = MinecraftVersionKind::LegacyRelease(LegacyReleaseVersion {
            major: 1,
            minor,
            patch,
            stage,
            stage_number,
        });
        let key = MinecraftVersionKey {
            era: VersionEra::Legacy,
            major: 1,
            minor,
            patch,
            stage,
            stage_number,
        };

        return Ok(MinecraftVersion::from_parts(raw, kind, key));
    }

    let year = full_year(first).ok_or_else(|| MinecraftVersionParseError::UnsupportedYear {
        raw: raw.to_owned(),
        year: first,
    })?;
    let release = numbers[1];
    let patch = numbers.get(2).copied().unwrap_or(0);

    if stage == VersionStage::Snapshot {
        let kind = MinecraftVersionKind::YearSnapshot(YearSnapshotVersion {
            year,
            release,
            patch,
            snapshot: stage_number,
        });
        let key = MinecraftVersionKey {
            era: VersionEra::Year,
            major: year,
            minor: release,
            patch,
            stage,
            stage_number,
        };

        return Ok(MinecraftVersion::from_parts(raw, kind, key));
    }

    let kind = MinecraftVersionKind::YearRelease(YearReleaseVersion {
        year,
        release,
        patch,
        stage,
        stage_number,
    });
    let key = MinecraftVersionKey {
        era: VersionEra::Year,
        major: year,
        minor: release,
        patch,
        stage,
        stage_number,
    };

    Ok(MinecraftVersion::from_parts(raw, kind, key))
}

fn parse_stage(raw: &str, stage: &str) -> Result<(VersionStage, u32), MinecraftVersionParseError> {
    if let Some(number) = stage.strip_prefix("snapshot-") {
        return Ok((VersionStage::Snapshot, parse_stage_number(raw, number)?));
    }
    if let Some(number) = stage.strip_prefix("pre") {
        return Ok((VersionStage::PreRelease, parse_stage_number(raw, number)?));
    }
    if let Some(number) = stage.strip_prefix("rc") {
        return Ok((
            VersionStage::ReleaseCandidate,
            parse_stage_number(raw, number)?,
        ));
    }

    Err(MinecraftVersionParseError::UnsupportedStage {
        raw: raw.to_owned(),
        stage: stage.to_owned(),
    })
}

fn parse_stage_number(raw: &str, value: &str) -> Result<u32, MinecraftVersionParseError> {
    value
        .parse()
        .map_err(|_| MinecraftVersionParseError::InvalidFormat {
            raw: raw.to_owned(),
            reason: "stage suffix must end with a number",
        })
}

fn parse_numeric_components(
    raw: &str,
    value: &str,
) -> Result<Vec<u32>, MinecraftVersionParseError> {
    value
        .split('.')
        .map(|component| parse_component(raw, component))
        .collect()
}

fn parse_component(raw: &str, component: &str) -> Result<u32, MinecraftVersionParseError> {
    component
        .parse()
        .map_err(|_| MinecraftVersionParseError::InvalidNumber {
            raw: raw.to_owned(),
            component: component.to_owned(),
        })
}

fn full_year(year: u32) -> Option<u32> {
    match year {
        25..=99 => Some(2000 + year),
        2000..=9999 => Some(year),
        _ => None,
    }
}

fn normalize_year(year: u32) -> u32 {
    if year < 100 { 2000 + year } else { year }
}

fn snapshot_suffix_rank(suffix: Option<char>) -> u32 {
    match suffix {
        Some(character) => u32::from(character) - u32::from('a') + 1,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{MinecraftVersion, MinecraftVersionKey, MinecraftVersionKind};

    fn key(raw: &str) -> MinecraftVersionKey {
        MinecraftVersion::parse(raw)
            .unwrap_or_else(|error| panic!("failed to parse '{raw}': {error}"))
            .key()
    }

    #[test]
    fn orders_legacy_components_numerically() {
        assert!(key("1.9") < key("1.10"));
        assert!(key("1.21") < key("1.21.1"));
        assert!(key("a1.7.3") < key("b1.2.6"));
    }

    #[test]
    fn orders_year_versions_after_legacy_versions() {
        assert!(key("1.21.4") < key("26.1"));
        assert!(key("26.1") < key("26.1.1"));
        assert_eq!(key("26.1"), key("26.1.0"));
    }

    #[test]
    fn orders_release_stages() {
        assert!(key("1.21.4-pre1") < key("1.21.4-pre10"));
        assert!(key("1.21.4-pre10") < key("1.21.4-rc1"));
        assert!(key("1.21.4-rc1") < key("1.21.4"));
        assert!(key("25.4-snapshot-1") < key("25.4"));
    }

    #[test]
    fn parses_legacy_snapshots() {
        let version = MinecraftVersion::parse("25w41a")
            .unwrap_or_else(|error| panic!("failed to parse legacy snapshot: {error}"));
        let MinecraftVersionKind::LegacySnapshot(snapshot) = version.kind() else {
            panic!("expected a legacy snapshot, got {:?}", version.kind());
        };

        assert_eq!(snapshot.year, 2025);
        assert_eq!(snapshot.week, 41);
        assert_eq!(snapshot.suffix, Some('a'));
    }

    #[test]
    fn parses_year_versions() {
        let version = MinecraftVersion::parse("26.1")
            .unwrap_or_else(|error| panic!("failed to parse year version: {error}"));
        let MinecraftVersionKind::YearRelease(release) = version.kind() else {
            panic!("expected a year release, got {:?}", version.kind());
        };

        assert_eq!(release.year, 2026);
        assert_eq!(release.release, 1);
        assert_eq!(release.patch, 0);
    }

    #[test]
    fn constructs_canonical_year_versions() {
        assert_eq!(MinecraftVersion::year_release(2026, 1, 0).raw(), "26.1");
        assert_eq!(MinecraftVersion::year_release(26, 1, 1).raw(), "26.1.1");
    }

    #[test]
    fn rejects_unknown_formats() {
        assert!(MinecraftVersion::parse("not-a-version").is_err());
    }
}
