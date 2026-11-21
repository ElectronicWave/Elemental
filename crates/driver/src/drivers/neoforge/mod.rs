use elemental_core::minecraft::{MinecraftVersion, MinecraftVersionId};

use crate::{
    catalog::{InstallerMavenCatalog, LoaderRelease},
    descriptors::NEOFORGE_DRIVER,
    families::installer::{
        InstallerFamilyDriver, InstallerMavenSource, MavenEndpoints, MavenInstallerSpec,
        merge_libraries_prefer_embedded,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
    url::Origin,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NeoForgeOrigin;

impl Origin for NeoForgeOrigin {
    fn canonical(self) -> &'static str {
        "https://maven.neoforged.net/releases"
    }

    fn all() -> &'static [Self] {
        &[Self]
    }
}

impl AsRef<str> for NeoForgeOrigin {
    fn as_ref(&self) -> &str {
        self.canonical()
    }
}

const NEOFORGE_MARKERS: LibraryPrefixSet = LibraryPrefixSet::new(&[
    "net.neoforged:neoforge:",
    "net.neoforged:forge:",
    "net.neoforged:fmlloader:",
    "net.neoforged.fancymodloader:",
]);
const NEOFORGE_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    NEOFORGE_DRIVER,
    NEOFORGE_MARKERS,
    NEOFORGE_MARKERS,
    ProfileIdPattern::loader_prefixed("neoforge-"),
);

pub const NEOFORGE_SPEC: MavenInstallerSpec = MavenInstallerSpec {
    identity: NEOFORGE_IDENTITY,
    family_name: "neoforge",
    source_name: "neoforge source",
    loader_version_prefix: "neoforge-",
    canonical_origin: "https://maven.neoforged.net/releases",
    group: "net.neoforged",
    artifact: "neoforge",
    fixed_game_version: None,
    format_version: neoforge_version,
    parse_release: neoforge_release,
    merge_libraries: merge_libraries_prefer_embedded,
};

pub fn endpoints() -> MavenEndpoints {
    MavenEndpoints::official(&NEOFORGE_SPEC)
}

pub fn source() -> InstallerMavenSource {
    InstallerMavenSource::official(&NEOFORGE_SPEC)
}

pub fn driver() -> InstallerFamilyDriver {
    InstallerFamilyDriver::for_spec(&NEOFORGE_SPEC)
}

pub fn catalog() -> InstallerMavenCatalog {
    InstallerMavenCatalog::for_spec(&NEOFORGE_SPEC)
}

fn neoforge_version(
    _game_version: &MinecraftVersionId,
    loader_version: &LoaderVersionId,
) -> String {
    loader_version.as_str().to_owned()
}

fn neoforge_release(
    _spec: &MavenInstallerSpec,
    version: String,
) -> Option<(MinecraftVersionId, LoaderRelease)> {
    let game_version = infer_neoforge_game_version(version.as_str())?;
    Some((
        game_version.clone(),
        LoaderRelease::new(game_version, LoaderVersionId::from(version)),
    ))
}

fn infer_neoforge_game_version(loader_version: &str) -> Option<MinecraftVersionId> {
    let core_version = loader_version.split('-').next().unwrap_or(loader_version);
    let parts = core_version.split('.').collect::<Vec<&str>>();
    let [major, minor, ..] = parts.as_slice() else {
        return None;
    };

    let major_number = major.parse::<u32>().ok()?;
    let minor_number = minor.parse::<u32>().ok()?;
    if major_number >= 26 {
        let patch_number = match parts.get(2) {
            Some(patch) => patch.parse::<u32>().ok()?,
            None => 0,
        };

        return Some(
            MinecraftVersion::year_release(major_number, minor_number, patch_number).into_id(),
        );
    }

    Some(MinecraftVersion::legacy_release(1, major_number, minor_number).into_id())
}
