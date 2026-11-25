use super::{
    common::{full_year, parse_numeric_components},
    stage::parse_stage,
};
use crate::minecraft::version::error::MinecraftVersionParseError;
use crate::minecraft::version::kind::{
    LegacyReleaseVersion, MinecraftVersionKind, VersionStage, YearReleaseVersion,
    YearSnapshotVersion,
};

struct YearComponents {
    year: u32,
    release: u32,
    patch: u32,
}

pub(crate) fn parse_release(raw: &str) -> Result<MinecraftVersionKind, MinecraftVersionParseError> {
    let (numbers, stage, stage_number) = release_components(raw)?;

    if numbers[0] == 1 {
        return legacy_release(raw, &numbers, stage, stage_number);
    }

    year_version(raw, &numbers, stage, stage_number)
}

fn release_components(
    raw: &str,
) -> Result<(Vec<u32>, VersionStage, u32), MinecraftVersionParseError> {
    let (base, stage_part) = match raw.split_once('-') {
        Some((base, stage)) => (base, Some(stage)),
        None => (raw, None),
    };
    let numbers = parse_numeric_components(raw, base)?;
    validate_component_count(raw, &numbers)?;
    let (stage, stage_number) = match stage_part {
        Some(stage) => parse_stage(raw, stage)?,
        None => (VersionStage::Release, 0),
    };

    Ok((numbers, stage, stage_number))
}

fn validate_component_count(raw: &str, numbers: &[u32]) -> Result<(), MinecraftVersionParseError> {
    if (2..=3).contains(&numbers.len()) {
        Ok(())
    } else {
        Err(MinecraftVersionParseError::InvalidFormat {
            raw: raw.to_owned(),
            reason: "version must have two or three numeric components",
        })
    }
}

fn legacy_release(
    raw: &str,
    numbers: &[u32],
    stage: VersionStage,
    stage_number: u32,
) -> Result<MinecraftVersionKind, MinecraftVersionParseError> {
    validate_legacy_stage(raw, stage)?;

    let version = LegacyReleaseVersion {
        major: 1,
        minor: numbers[1],
        patch: numbers.get(2).copied().unwrap_or(0),
        stage,
        stage_number,
    };

    Ok(MinecraftVersionKind::LegacyRelease(version))
}

fn validate_legacy_stage(raw: &str, stage: VersionStage) -> Result<(), MinecraftVersionParseError> {
    if stage == VersionStage::Snapshot {
        return Err(MinecraftVersionParseError::InvalidFormat {
            raw: raw.to_owned(),
            reason: "legacy releases cannot use a snapshot stage",
        });
    }

    Ok(())
}

fn year_version(
    raw: &str,
    numbers: &[u32],
    stage: VersionStage,
    stage_number: u32,
) -> Result<MinecraftVersionKind, MinecraftVersionParseError> {
    let components = year_components(raw, numbers)?;
    if stage == VersionStage::Snapshot {
        return Ok(year_snapshot(components, stage_number));
    }

    Ok(year_release(components, stage, stage_number))
}

fn year_components(
    raw: &str,
    numbers: &[u32],
) -> Result<YearComponents, MinecraftVersionParseError> {
    let year =
        full_year(numbers[0]).ok_or_else(|| MinecraftVersionParseError::UnsupportedYear {
            raw: raw.to_owned(),
            year: numbers[0],
        })?;

    Ok(YearComponents {
        year,
        release: numbers[1],
        patch: numbers.get(2).copied().unwrap_or(0),
    })
}

fn year_snapshot(components: YearComponents, snapshot: u32) -> MinecraftVersionKind {
    let version = YearSnapshotVersion {
        year: components.year,
        release: components.release,
        patch: components.patch,
        snapshot,
    };

    MinecraftVersionKind::YearSnapshot(version)
}

fn year_release(
    components: YearComponents,
    stage: VersionStage,
    stage_number: u32,
) -> MinecraftVersionKind {
    let version = YearReleaseVersion {
        year: components.year,
        release: components.release,
        patch: components.patch,
        stage,
        stage_number,
    };

    MinecraftVersionKind::YearRelease(version)
}
