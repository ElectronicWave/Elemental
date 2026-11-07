use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    catalog::{InstallerMavenCatalog, LoaderRelease},
    descriptors::NEOFORGE_DRIVER,
    families::installer::{
        InstallerFamily, InstallerFamilyDriver, InstallerMavenSource, MavenEndpoints, MavenSpec,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
    url::Origin,
};

const GAME_VERSION_HEURISTIC_DESCRIPTION: &str = "Game version is inferred from NeoForge version naming. Installer profile remains the source of truth.";

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

#[derive(Debug, Clone, Copy)]
pub struct NeoForgeSpec;

impl MavenSpec for NeoForgeSpec {
    type Origin = NeoForgeOrigin;

    const SOURCE_NAME: &'static str = "neoforge source";
    const MAVEN_ORIGIN: NeoForgeOrigin = NeoForgeOrigin;
    const GROUP_PATH: &'static str = "net/neoforged";
    const COORDINATE_GROUP: &'static str = "net.neoforged";
    const ARTIFACT: &'static str = "neoforge";

    fn version(_game_version: &MinecraftVersionId, loader_version: &LoaderVersionId) -> String {
        loader_version.as_str().to_owned()
    }

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)> {
        let game_version = infer_neoforge_game_version(version.as_str())?;
        Some((
            game_version.clone(),
            LoaderRelease::new(game_version, LoaderVersionId::from(version))
                .with_description(Some(GAME_VERSION_HEURISTIC_DESCRIPTION.to_owned())),
        ))
    }
}

pub type NeoForgeEndpoints = MavenEndpoints<NeoForgeSpec>;
pub type NeoForgeSource = InstallerMavenSource<NeoForgeEndpoints>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NeoForgeFamily;

impl InstallerFamily for NeoForgeFamily {
    type Source = NeoForgeSource;
    type Endpoints = NeoForgeEndpoints;

    const IDENTITY: ProfiledDriverIdentity = NEOFORGE_IDENTITY;
    const FAMILY_NAME: &'static str = "neoforge";
    const LOADER_VERSION_PREFIX: &'static str = "neoforge-";
}

pub type NeoForgeDriver = InstallerFamilyDriver<NeoForgeFamily>;

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

pub type NeoForgeCatalog = InstallerMavenCatalog<NeoForgeEndpoints>;

fn infer_neoforge_game_version(loader_version: &str) -> Option<MinecraftVersionId> {
    let core_version = loader_version.split('-').next().unwrap_or(loader_version);
    let parts = core_version.split('.').collect::<Vec<&str>>();
    let [major, minor, ..] = parts.as_slice() else {
        return None;
    };

    let major_number = major.parse::<u32>().ok()?;
    if major_number >= 26 {
        let game_version = match parts.get(2).copied() {
            Some("0") | None => format!("{major}.{minor}"),
            Some(hotfix) => format!("{major}.{minor}.{hotfix}"),
        };
        return Some(MinecraftVersionId::from(game_version));
    }

    Some(MinecraftVersionId::from(format!("1.{major}.{minor}")))
}
