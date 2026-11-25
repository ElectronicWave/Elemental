use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    catalog::LoaderRelease,
    descriptors::FORGE_DRIVER,
    driver::InstallerFamilyDriver,
    families::installer::{
        InstallerMavenCatalog, InstallerMavenSource, MavenEndpoints, MavenInstallerSpec,
        merge_libraries_prefer_embedded,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
    url::Origin,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ForgeOrigin;

impl Origin for ForgeOrigin {
    fn canonical(self) -> &'static str {
        "https://maven.minecraftforge.net"
    }

    fn all() -> &'static [Self] {
        &[Self]
    }
}

impl AsRef<str> for ForgeOrigin {
    fn as_ref(&self) -> &str {
        self.canonical()
    }
}

const FORGE_LIBRARIES: LibraryPrefixSet = LibraryPrefixSet::new(&["net.minecraftforge:fmlloader:"]);
const FORGE_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    FORGE_DRIVER,
    FORGE_LIBRARIES,
    FORGE_LIBRARIES,
    ProfileIdPattern::new("-forge-"),
);

pub const FORGE_SPEC: MavenInstallerSpec = MavenInstallerSpec {
    identity: FORGE_IDENTITY,
    family_name: "forge",
    source_name: "forge source",
    loader_version_prefix: "forge-",
    canonical_origin: "https://maven.minecraftforge.net",
    group: "net.minecraftforge",
    artifact: "forge",
    fixed_game_version: None,
    format_version: forge_version,
    parse_release: forge_release,
    merge_libraries: merge_libraries_prefer_embedded,
};

pub fn endpoints() -> MavenEndpoints {
    MavenEndpoints::official(&FORGE_SPEC)
}

pub fn source() -> InstallerMavenSource {
    InstallerMavenSource::official(&FORGE_SPEC)
}

pub fn driver() -> InstallerFamilyDriver {
    InstallerFamilyDriver::for_spec(&FORGE_SPEC)
}

pub fn catalog() -> InstallerMavenCatalog {
    InstallerMavenCatalog::for_spec(&FORGE_SPEC)
}

fn forge_version(game_version: &MinecraftVersionId, loader_version: &LoaderVersionId) -> String {
    format!("{game_version}-{loader_version}")
}

fn forge_release(
    _spec: &MavenInstallerSpec,
    version: String,
) -> Option<(MinecraftVersionId, LoaderRelease)> {
    let mut parts = version.split('-');
    let game_version = parts.next()?;
    let loader_version = parts.next()?;
    let game_version = MinecraftVersionId::from(game_version);
    Some((
        game_version.clone(),
        LoaderRelease::new(game_version, LoaderVersionId::from(loader_version)),
    ))
}
