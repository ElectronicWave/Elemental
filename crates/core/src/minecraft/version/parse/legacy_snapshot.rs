use super::common::parse_component;
use crate::minecraft::version::kind::{LegacySnapshotVersion, MinecraftVersionKind};
use crate::minecraft::version::{MinecraftVersion, MinecraftVersionParseError};

pub(crate) fn parse_legacy_snapshot(
    raw: &str,
) -> Result<Option<MinecraftVersion>, MinecraftVersionParseError> {
    let (year_part, week_part, suffix_part) = match split_legacy_snapshot(raw) {
        Some(parts) => parts,
        None => return Ok(None),
    };

    let year = 2000 + parse_component(raw, year_part)?;
    let week = parse_component(raw, week_part)?;
    let suffix = suffix_part
        .chars()
        .next()
        .map(|character| character.to_ascii_lowercase());
    let kind = MinecraftVersionKind::LegacySnapshot(LegacySnapshotVersion { year, week, suffix });
    Ok(Some(MinecraftVersion::from_kind(raw, kind)))
}

fn split_legacy_snapshot(raw: &str) -> Option<(&str, &str, &str)> {
    let (year_part, rest) = raw.split_once('w')?;
    if !is_two_digit_year(year_part) {
        return None;
    }

    let digit_count = rest
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digit_count == 0 {
        return None;
    }

    let week_part = &rest[..digit_count];
    let suffix_part = &rest[digit_count..];
    if !is_valid_suffix(suffix_part) {
        return None;
    }

    Some((year_part, week_part, suffix_part))
}

fn is_two_digit_year(value: &str) -> bool {
    value.len() == 2 && value.chars().all(|character| character.is_ascii_digit())
}

fn is_valid_suffix(value: &str) -> bool {
    value.chars().count() <= 1
        && value
            .chars()
            .all(|character| character.is_ascii_alphabetic())
}
