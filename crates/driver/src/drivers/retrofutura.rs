use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    catalog::LoaderRelease,
    families::installer::{MavenInstallerSpec, merge_retro_futura_libraries},
    inspect::ProfiledDriverIdentity,
    loader_version::LoaderVersionId,
};

/// Metadata describing a RetroFutura-style loader line that replaces the
/// runtime of an old Minecraft version while keeping that version's game
/// metadata as the base.
#[derive(Debug, Clone, Copy)]
pub struct RetroFuturaLine {
    pub identity: ProfiledDriverIdentity,
    pub family_name: &'static str,
    pub source_name: &'static str,
    pub loader_version_prefix: &'static str,
    pub canonical_origin: &'static str,
    pub group: &'static str,
    pub artifact: &'static str,
    pub game_version: &'static str,
}

/// Builds a [`MavenInstallerSpec`] for a loader line whose runtime replaces a
/// fixed base Minecraft version while keeping its game metadata.
pub const fn retrofutura_spec(line: RetroFuturaLine) -> MavenInstallerSpec {
    MavenInstallerSpec {
        identity: line.identity,
        family_name: line.family_name,
        source_name: line.source_name,
        loader_version_prefix: line.loader_version_prefix,
        canonical_origin: line.canonical_origin,
        group: line.group,
        artifact: line.artifact,
        fixed_game_version: Some(line.game_version),
        format_version: retrofutura_version,
        parse_release: retrofutura_release,
        merge_libraries: merge_retro_futura_libraries,
    }
}

fn retrofutura_version(
    _game_version: &MinecraftVersionId,
    loader_version: &LoaderVersionId,
) -> String {
    loader_version.as_str().to_owned()
}

fn retrofutura_release(
    spec: &MavenInstallerSpec,
    version: String,
) -> Option<(MinecraftVersionId, LoaderRelease)> {
    let game_version = MinecraftVersionId::from(spec.fixed_game_version?);
    Some((
        game_version.clone(),
        LoaderRelease::new(game_version, LoaderVersionId::from(version)),
    ))
}
