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
