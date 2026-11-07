use super::common::parse_numeric_components;
use crate::minecraft::version::kind::{
    AlphaBetaVersion, MinecraftVersionKey, MinecraftVersionKind, VersionStage,
};
use crate::minecraft::version::{MinecraftVersion, MinecraftVersionParseError};

pub(crate) fn parse_alpha_beta(
    raw: &str,
) -> Result<Option<MinecraftVersion>, MinecraftVersionParseError> {
    let stage = match alpha_beta_stage(raw) {
        Some(stage) => stage,
        None => return Ok(None),
    };
    let numeric = &raw[1..];
    if !numeric.starts_with(|character: char| character.is_ascii_digit()) {
        return Ok(None);
    }

    let numbers = parse_numeric_components(raw, numeric)?;
    let (major, minor, patch) = alpha_beta_components(raw, &numbers)?;
    let kind = MinecraftVersionKind::AlphaBeta(AlphaBetaVersion {
        stage,
        major,
        minor,
        patch,
    });
    let key = MinecraftVersionKey::alpha_beta(stage, major, minor, patch);

    Ok(Some(MinecraftVersion::from_parts(raw, kind, key)))
}

fn alpha_beta_stage(raw: &str) -> Option<VersionStage> {
    match raw.chars().next() {
        Some('a') => Some(VersionStage::Alpha),
        Some('b') => Some(VersionStage::Beta),
        _ => None,
    }
}

fn alpha_beta_components(
    raw: &str,
    numbers: &[u32],
) -> Result<(u32, u32, u32), MinecraftVersionParseError> {
    match numbers {
        [major, minor] => Ok((*major, *minor, 0)),
        [major, minor, patch] => Ok((*major, *minor, *patch)),
        _ => Err(MinecraftVersionParseError::InvalidFormat {
            raw: raw.to_owned(),
            reason: "alpha/beta version must have two or three numeric components",
        }),
    }
}
