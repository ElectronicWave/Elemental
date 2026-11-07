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
    pub(super) era: VersionEra,
    pub(super) major: u32,
    pub(super) minor: u32,
    pub(super) patch: u32,
    pub(super) stage: VersionStage,
    pub(super) stage_number: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum VersionEra {
    Alpha,
    Beta,
    Legacy,
    Year,
}
