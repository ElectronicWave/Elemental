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
    fn alpha_beta(version: AlphaBetaVersion) -> Self {
        let era = match version.stage {
            VersionStage::Alpha => VersionEra::Alpha,
            _ => VersionEra::Beta,
        };

        Self {
            era,
            major: version.major,
            minor: version.minor,
            patch: version.patch,
            stage: version.stage,
            stage_number: 0,
        }
    }

    fn legacy_snapshot(version: LegacySnapshotVersion) -> Self {
        Self {
            era: VersionEra::Legacy,
            major: version.year,
            minor: version.week,
            patch: 0,
            stage: VersionStage::Snapshot,
            stage_number: snapshot_suffix_rank(version.suffix),
        }
    }

    fn legacy_release(version: LegacyReleaseVersion) -> Self {
        Self {
            era: VersionEra::Legacy,
            major: version.major,
            minor: version.minor,
            patch: version.patch,
            stage: version.stage,
            stage_number: version.stage_number,
        }
    }

    fn year_snapshot(version: YearSnapshotVersion) -> Self {
        Self {
            era: VersionEra::Year,
            major: version.year,
            minor: version.release,
            patch: version.patch,
            stage: VersionStage::Snapshot,
            stage_number: version.snapshot,
        }
    }

    fn year_release(version: YearReleaseVersion) -> Self {
        Self {
            era: VersionEra::Year,
            major: version.year,
            minor: version.release,
            patch: version.patch,
            stage: version.stage,
            stage_number: version.stage_number,
        }
    }
}

impl From<MinecraftVersionKind> for MinecraftVersionKey {
    fn from(kind: MinecraftVersionKind) -> Self {
        match kind {
            MinecraftVersionKind::AlphaBeta(version) => Self::alpha_beta(version),
            MinecraftVersionKind::LegacySnapshot(version) => Self::legacy_snapshot(version),
            MinecraftVersionKind::LegacyRelease(version) => Self::legacy_release(version),
            MinecraftVersionKind::YearSnapshot(version) => Self::year_snapshot(version),
            MinecraftVersionKind::YearRelease(version) => Self::year_release(version),
        }
    }
}

fn snapshot_suffix_rank(suffix: Option<char>) -> u32 {
    match suffix {
        Some(character) => u32::from(character) - u32::from('a') + 1,
        None => 0,
    }
}
