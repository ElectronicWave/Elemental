#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MinecraftVersionKind {
    AlphaBeta(AlphaBetaVersion),
    LegacySnapshot(LegacySnapshotVersion),
    LegacyRelease(LegacyReleaseVersion),
    YearSnapshot(YearSnapshotVersion),
    YearRelease(YearReleaseVersion),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AlphaBetaVersion {
    pub stage: VersionStage,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LegacySnapshotVersion {
    /// Full year, for example `2025` for `25w41a`.
    pub year: u32,
    pub week: u32,
    pub suffix: Option<char>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LegacyReleaseVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub stage: VersionStage,
    pub stage_number: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct YearSnapshotVersion {
    /// Full year, for example `2025` for `25.4-snapshot-1`.
    pub year: u32,
    pub release: u32,
    pub patch: u32,
    pub snapshot: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct YearReleaseVersion {
    /// Full year, for example `2026` for `26.1`.
    pub year: u32,
    pub release: u32,
    pub patch: u32,
    pub stage: VersionStage,
    pub stage_number: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VersionStage {
    Alpha,
    Beta,
    Snapshot,
    PreRelease,
    ReleaseCandidate,
    Release,
}

/// Ordering key for Minecraft versions.
///
/// The key normalizes missing components, so `1.21` and `1.21.0` have the same
/// semantic position in a sorted collection. The original identifier is never
/// changed. Legacy releases sort before year-based releases; snapshots are
/// ordered by their own naming coordinates, so comparisons across snapshot
/// naming schemes are approximate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MinecraftVersionKey {
    era: VersionEra,
    major: u32,
    minor: u32,
    patch: u32,
    stage: VersionStage,
    stage_number: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum VersionEra {
    Alpha,
    Beta,
    Legacy,
    Year,
}

impl MinecraftVersionKey {
    pub(super) fn alpha_beta(stage: VersionStage, major: u32, minor: u32, patch: u32) -> Self {
        let era = match stage {
            VersionStage::Alpha => VersionEra::Alpha,
            _ => VersionEra::Beta,
        };

        Self {
            era,
            major,
            minor,
            patch,
            stage,
            stage_number: 0,
        }
    }

    pub(super) fn legacy_snapshot(year: u32, week: u32, suffix: Option<char>) -> Self {
        Self {
            era: VersionEra::Legacy,
            major: year,
            minor: week,
            patch: 0,
            stage: VersionStage::Snapshot,
            stage_number: snapshot_suffix_rank(suffix),
        }
    }

    pub(super) fn legacy_release(
        major: u32,
        minor: u32,
        patch: u32,
        stage: VersionStage,
        stage_number: u32,
    ) -> Self {
        Self {
            era: VersionEra::Legacy,
            major,
            minor,
            patch,
            stage,
            stage_number,
        }
    }

    pub(super) fn year_snapshot(year: u32, release: u32, patch: u32, snapshot: u32) -> Self {
        Self {
            era: VersionEra::Year,
            major: year,
            minor: release,
            patch,
            stage: VersionStage::Snapshot,
            stage_number: snapshot,
        }
    }

    pub(super) fn year_release(
        year: u32,
        release: u32,
        patch: u32,
        stage: VersionStage,
        stage_number: u32,
    ) -> Self {
        Self {
            era: VersionEra::Year,
            major: year,
            minor: release,
            patch,
            stage,
            stage_number,
        }
    }
}

fn snapshot_suffix_rank(suffix: Option<char>) -> u32 {
    match suffix {
        Some(character) => u32::from(character) - u32::from('a') + 1,
        None => 0,
    }
}
