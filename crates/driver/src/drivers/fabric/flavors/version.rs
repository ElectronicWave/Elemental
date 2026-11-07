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
    parse_release_minor(game_version).is_some_and(|minor| minor >= 14)
        || parse_snapshot_version(game_version).is_some_and(is_modern_fabric_snapshot)
}

pub(super) fn is_legacy_fabric_game_version(game_version: &str) -> bool {
    parse_release_minor(game_version).is_some_and(|minor| minor < 14)
        || parse_snapshot_version(game_version)
            .is_some_and(|snapshot| !is_modern_fabric_snapshot(snapshot))
}

fn parse_release_minor(game_version: &str) -> Option<u32> {
    let mut segments = game_version.split('.');
    let major = segments.next()?;
    if major != "1" {
        return None;
    }

    segments.next()?.parse::<u32>().ok()
}

fn parse_snapshot_version(game_version: &str) -> Option<(u32, u32, Option<char>)> {
    let (year, rest) = game_version.split_once('w')?;
    let year = year.parse::<u32>().ok()?;
    let digit_count = rest
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digit_count == 0 {
        return None;
    }

    let week = rest[..digit_count].parse::<u32>().ok()?;
    let suffix = rest[digit_count..].chars().next();

    Some((year, week, suffix))
}

fn is_modern_fabric_snapshot(snapshot: (u32, u32, Option<char>)) -> bool {
    let (year, week, suffix) = snapshot;
    if year > 18 {
        return true;
    }
    if year < 18 {
        return false;
    }
    if week > 43 {
        return true;
    }
    if week < 43 {
        return false;
    }

    suffix.is_none_or(|letter| letter >= 'b')
}
