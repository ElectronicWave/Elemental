use super::{
    common::{full_year, parse_numeric_components},
    stage::parse_stage,
};
use crate::minecraft::version::kind::{
    LegacyReleaseVersion, MinecraftVersionKey, MinecraftVersionKind, VersionStage,
    YearReleaseVersion, YearSnapshotVersion,
};
use crate::minecraft::version::{MinecraftVersion, MinecraftVersionParseError};

pub(crate) fn parse_release(raw: &str) -> Result<MinecraftVersion, MinecraftVersionParseError> {
    let (numbers, stage, stage_number) = release_components(raw)?;

    if numbers[0] == 1 {
        return legacy_release(raw, &numbers, stage, stage_number);
    }

    year_version(raw, &numbers, stage, stage_number)
}

fn release_components(
    raw: &str,
) -> Result<(Vec<u32>, VersionStage, u32), MinecraftVersionParseError> {
    let (base, stage_part) = split_stage(raw);
    let numbers = parse_numeric_components(raw, base)?;
    ensure_component_count(raw, &numbers)?;
    let (stage, stage_number) = parse_release_stage(raw, stage_part)?;

    Ok((numbers, stage, stage_number))
}

fn split_stage(raw: &str) -> (&str, Option<&str>) {
    match raw.split_once('-') {
        Some((base, stage)) => (base, Some(stage)),
        None => (raw, None),
    }
}

fn ensure_component_count(raw: &str, numbers: &[u32]) -> Result<(), MinecraftVersionParseError> {
    if (2..=3).contains(&numbers.len()) {
        Ok(())
    } else {
        Err(MinecraftVersionParseError::InvalidFormat {
            raw: raw.to_owned(),
            reason: "version must have two or three numeric components",
        })
    }
}

fn parse_release_stage(
    raw: &str,
    stage_part: Option<&str>,
) -> Result<(VersionStage, u32), MinecraftVersionParseError> {
    match stage_part {
        Some(stage) => parse_stage(raw, stage),
        None => Ok((VersionStage::Release, 0)),
    }
}

fn legacy_release(
    raw: &str,
    numbers: &[u32],
    stage: VersionStage,
    stage_number: u32,
) -> Result<MinecraftVersion, MinecraftVersionParseError> {
    if stage == VersionStage::Snapshot {
        return Err(MinecraftVersionParseError::InvalidFormat {
            raw: raw.to_owned(),
            reason: "legacy releases cannot use a snapshot stage",
        });
    }

    let version = LegacyReleaseVersion {
        major: 1,
        minor: numbers[1],
        patch: numbers.get(2).copied().unwrap_or(0),
        stage,
        stage_number,
    };
    let key = MinecraftVersionKey::legacy_release(
        version.major,
        version.minor,
        version.patch,
        version.stage,
        version.stage_number,
    );

    Ok(MinecraftVersion::from_parts(
        raw,
        MinecraftVersionKind::LegacyRelease(version),
        key,
    ))
}

fn year_version(
    raw: &str,
    numbers: &[u32],
    stage: VersionStage,
    stage_number: u32,
) -> Result<MinecraftVersion, MinecraftVersionParseError> {
    let year =
        full_year(numbers[0]).ok_or_else(|| MinecraftVersionParseError::UnsupportedYear {
            raw: raw.to_owned(),
            year: numbers[0],
        })?;
    let release = numbers[1];
    let patch = numbers.get(2).copied().unwrap_or(0);

    if stage == VersionStage::Snapshot {
        let version = YearSnapshotVersion {
            year,
            release,
            patch,
            snapshot: stage_number,
        };
        return Ok(year_snapshot(raw, version));
    }

    let version = YearReleaseVersion {
        year,
        release,
        patch,
        stage,
        stage_number,
    };

    Ok(year_release(raw, version))
}

fn year_snapshot(raw: &str, version: YearSnapshotVersion) -> MinecraftVersion {
    let key = MinecraftVersionKey::year_snapshot(
        version.year,
        version.release,
        version.patch,
        version.snapshot,
    );

    MinecraftVersion::from_parts(raw, MinecraftVersionKind::YearSnapshot(version), key)
}

fn year_release(raw: &str, version: YearReleaseVersion) -> MinecraftVersion {
    let key = MinecraftVersionKey::year_release(
        version.year,
        version.release,
        version.patch,
        version.stage,
        version.stage_number,
    );

    MinecraftVersion::from_parts(raw, MinecraftVersionKind::YearRelease(version), key)
}
