use crate::minecraft::version::MinecraftVersionParseError;
use crate::minecraft::version::kind::VersionStage;

pub(super) fn parse_stage(
    raw: &str,
    stage: &str,
) -> Result<(VersionStage, u32), MinecraftVersionParseError> {
    let (version_stage, number) =
        stage_kind(stage).ok_or_else(|| MinecraftVersionParseError::UnsupportedStage {
            raw: raw.to_owned(),
            stage: stage.to_owned(),
        })?;

    Ok((version_stage, parse_stage_number(raw, number)?))
}

fn stage_kind(stage: &str) -> Option<(VersionStage, &str)> {
    if let Some(number) = stage.strip_prefix("snapshot-") {
        return Some((VersionStage::Snapshot, number));
    }
    if let Some(number) = stage.strip_prefix("pre") {
        return Some((VersionStage::PreRelease, number));
    }

    stage
        .strip_prefix("rc")
        .map(|number| (VersionStage::ReleaseCandidate, number))
}

fn parse_stage_number(raw: &str, value: &str) -> Result<u32, MinecraftVersionParseError> {
    value
        .parse()
        .map_err(|_| MinecraftVersionParseError::InvalidFormat {
            raw: raw.to_owned(),
            reason: "stage suffix must end with a number",
        })
}
