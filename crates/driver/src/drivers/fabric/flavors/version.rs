use elemental_core::minecraft::{
    LegacyReleaseVersion, LegacySnapshotVersion, MinecraftVersion, MinecraftVersionKind,
};
use elemental_schema::mojang::piston::PistonMetaData;

pub(super) fn metadata_has_babric_markers(metadata: &PistonMetaData) -> bool {
    metadata.libraries.iter().any(|library| {
        library.name.starts_with("babric:")
            || library
                .name
                .starts_with("org.lwjgl.lwjgl:lwjgl:2.9.4-babric.")
            || library
                .name
                .starts_with("org.lwjgl.lwjgl:lwjgl_util:2.9.4-babric.")
            || library
                .name
                .starts_with("org.lwjgl.lwjgl:lwjgl-platform:2.9.4-babric.")
    })
}

pub(super) fn is_modern_fabric_game_version(game_version: &str) -> bool {
    classify_fabric_game_version(game_version) == Some(FabricGameVersionClass::Modern)
}

pub(super) fn is_legacy_fabric_game_version(game_version: &str) -> bool {
    classify_fabric_game_version(game_version) == Some(FabricGameVersionClass::Legacy)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FabricGameVersionClass {
    Modern,
    Legacy,
}

fn classify_fabric_game_version(game_version: &str) -> Option<FabricGameVersionClass> {
    let version = MinecraftVersion::parse(game_version).ok()?;

    match version.kind() {
        MinecraftVersionKind::LegacyRelease(release) => Some(classify_legacy_release(release)),
        MinecraftVersionKind::LegacySnapshot(snapshot) => Some(classify_legacy_snapshot(snapshot)),
        MinecraftVersionKind::YearRelease(_) | MinecraftVersionKind::YearSnapshot(_) => {
            Some(FabricGameVersionClass::Modern)
        }
        MinecraftVersionKind::AlphaBeta(_) => None,
    }
}

fn classify_legacy_release(release: &LegacyReleaseVersion) -> FabricGameVersionClass {
    if release.minor >= 14 {
        FabricGameVersionClass::Modern
    } else {
        FabricGameVersionClass::Legacy
    }
}

fn classify_legacy_snapshot(snapshot: &LegacySnapshotVersion) -> FabricGameVersionClass {
    if is_modern_fabric_snapshot(snapshot) {
        FabricGameVersionClass::Modern
    } else {
        FabricGameVersionClass::Legacy
    }
}

fn is_modern_fabric_snapshot(snapshot: &LegacySnapshotVersion) -> bool {
    if snapshot.year > 2018 {
        return true;
    }
    if snapshot.year < 2018 {
        return false;
    }
    if snapshot.week > 43 {
        return true;
    }
    if snapshot.week < 43 {
        return false;
    }

    snapshot.suffix.is_none_or(|letter| letter >= 'b')
}
