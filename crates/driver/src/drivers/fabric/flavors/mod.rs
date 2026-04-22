mod babric;
mod common;
mod passthrough;

use anyhow::Result;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::fabric::ProfileJson;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::loader_version::LoaderVersionId;
use crate::{driver::DriverDescriptor, drivers::fabric::source::FabricFlavor};

use self::common::FlavorBehavior;

const FABRIC_META_ORIGIN: &str = "https://meta.fabricmc.net";
const FABRIC_MAVEN_ORIGIN: &str = "https://maven.fabricmc.net";
const LEGACY_FABRIC_META_ORIGIN: &str = "https://meta.legacyfabric.net";
const LEGACY_FABRIC_MAVEN_ORIGIN: &str = "https://maven.legacyfabric.net";
const BABRIC_META_ORIGIN: &str = "https://meta.babric.glass-launcher.net";
const BABRIC_MAVEN_ORIGIN: &str = "https://maven.glass-launcher.net/babric";

const FABRIC_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "fabric",
    name: "Fabric",
};
const LEGACY_FABRIC_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "legacyfabric",
    name: "LegacyFabric",
};
const BABRIC_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "babric",
    name: "Babric",
};

const FABRIC_LOADER_PREFIXES: &[&str] = &["net.fabricmc:fabric-loader:"];
const LEGACY_FABRIC_LOADER_PREFIXES: &[&str] = &[
    "net.fabricmc:fabric-loader:",
    "net.legacyfabric:fabric-loader:",
];
const BABRIC_LOADER_PREFIXES: &[&str] = &["net.fabricmc:fabric-loader:"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FabricFlavorKind {
    Fabric,
    LegacyFabric,
    Babric,
}

pub(crate) struct FabricFlavorSpec {
    kind: FabricFlavorKind,
    descriptor: DriverDescriptor,
    meta_origin: &'static str,
    maven_origin: &'static str,
    loader_prefixes: &'static [&'static str],
    behavior: &'static dyn FlavorBehavior,
}

const FABRIC_SPEC: FabricFlavorSpec = FabricFlavorSpec {
    kind: FabricFlavorKind::Fabric,
    descriptor: FABRIC_DRIVER,
    meta_origin: FABRIC_META_ORIGIN,
    maven_origin: FABRIC_MAVEN_ORIGIN,
    loader_prefixes: FABRIC_LOADER_PREFIXES,
    behavior: &passthrough::BEHAVIOR,
};

const LEGACY_FABRIC_SPEC: FabricFlavorSpec = FabricFlavorSpec {
    kind: FabricFlavorKind::LegacyFabric,
    descriptor: LEGACY_FABRIC_DRIVER,
    meta_origin: LEGACY_FABRIC_META_ORIGIN,
    maven_origin: LEGACY_FABRIC_MAVEN_ORIGIN,
    loader_prefixes: LEGACY_FABRIC_LOADER_PREFIXES,
    behavior: &passthrough::BEHAVIOR,
};

const BABRIC_SPEC: FabricFlavorSpec = FabricFlavorSpec {
    kind: FabricFlavorKind::Babric,
    descriptor: BABRIC_DRIVER,
    meta_origin: BABRIC_META_ORIGIN,
    maven_origin: BABRIC_MAVEN_ORIGIN,
    loader_prefixes: BABRIC_LOADER_PREFIXES,
    behavior: &babric::BEHAVIOR,
};

impl FabricFlavorSpec {
    pub(crate) fn descriptor(&self) -> DriverDescriptor {
        self.descriptor
    }

    pub(crate) fn meta_origin(&self) -> &str {
        self.meta_origin
    }

    pub(crate) fn maven_origin(&self) -> &str {
        self.maven_origin
    }

    pub(crate) fn inspect_driver_version(&self, metadata: &PistonMetaData) -> Option<String> {
        if !self.matches_metadata(metadata) {
            return None;
        }

        metadata
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .find(|name| {
                self.loader_prefixes
                    .iter()
                    .any(|prefix| name.starts_with(prefix))
            })
            .and_then(|name| name.split(':').nth(2).map(ToOwned::to_owned))
    }

    pub(crate) fn merge_profile(
        &self,
        base_metadata: PistonMetaData,
        profile: ProfileJson,
    ) -> Result<PistonMetaData> {
        common::merge_profile(self.behavior, base_metadata, profile)
    }

    pub(crate) fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool {
        self.behavior
            .local_metadata_needs_refresh(metadata, game_version, loader_version)
    }

    fn matches_metadata(&self, metadata: &PistonMetaData) -> bool {
        let has_loader = metadata.libraries.iter().any(|library| {
            self.loader_prefixes
                .iter()
                .any(|prefix| library.name.starts_with(prefix))
        });
        if !has_loader {
            return false;
        }

        let base_game_version = metadata
            .inherits_from
            .as_deref()
            .unwrap_or(metadata.id.as_str());

        match self.kind {
            FabricFlavorKind::Fabric => {
                !metadata_has_babric_markers(metadata)
                    && is_modern_fabric_game_version(base_game_version)
            }
            FabricFlavorKind::LegacyFabric => {
                !metadata_has_babric_markers(metadata)
                    && is_legacy_fabric_game_version(base_game_version)
            }
            FabricFlavorKind::Babric => metadata_has_babric_markers(metadata),
        }
    }
}

pub(crate) fn flavor_spec(flavor: &FabricFlavor) -> &'static FabricFlavorSpec {
    match flavor {
        FabricFlavor::Fabric => &FABRIC_SPEC,
        FabricFlavor::LegacyFabric => &LEGACY_FABRIC_SPEC,
        FabricFlavor::Babric => &BABRIC_SPEC,
    }
}

fn metadata_has_babric_markers(metadata: &PistonMetaData) -> bool {
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

fn is_modern_fabric_game_version(game_version: &str) -> bool {
    parse_release_minor(game_version).is_some_and(|minor| minor >= 14)
        || parse_snapshot_version(game_version).is_some_and(is_modern_fabric_snapshot)
}

fn is_legacy_fabric_game_version(game_version: &str) -> bool {
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
