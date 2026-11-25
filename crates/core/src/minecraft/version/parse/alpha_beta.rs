use super::common::parse_numeric_components;
use crate::minecraft::version::error::MinecraftVersionParseError;
use crate::minecraft::version::kind::{AlphaBetaVersion, MinecraftVersionKind, VersionStage};

pub(crate) fn parse_alpha_beta(
    raw: &str,
) -> Result<Option<MinecraftVersionKind>, MinecraftVersionParseError> {
    let stage = match alpha_beta_stage(raw) {
        Some(stage) => stage,
        None => return Ok(None),
    };
    let numeric = &raw[1..];
    if !numeric.starts_with(|character: char| character.is_ascii_digit()) {
        return Ok(None);
    }

    let numbers = parse_numeric_components(raw, numeric)?;
    let version = alpha_beta_components(raw, stage, &numbers)?;

    Ok(Some(MinecraftVersionKind::AlphaBeta(version)))
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
    stage: VersionStage,
    numbers: &[u32],
) -> Result<AlphaBetaVersion, MinecraftVersionParseError> {
    let (major, minor, patch) = match numbers {
        [major, minor] => (*major, *minor, 0),
        [major, minor, patch] => (*major, *minor, *patch),
        _ => return Err(invalid_format(raw)),
    };

    Ok(AlphaBetaVersion {
        stage,
        major,
        minor,
        patch,
    })
}

fn invalid_format(raw: &str) -> MinecraftVersionParseError {
    MinecraftVersionParseError::InvalidFormat {
        raw: raw.to_owned(),
        reason: "alpha/beta version must have two or three numeric components",
    }
}
