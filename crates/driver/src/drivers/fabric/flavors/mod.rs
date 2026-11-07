mod babric;
mod common;
mod passthrough;
mod version;

use crate::error::Result;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::fabric::ProfileJson;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::inspect::LibraryPrefixSet;
use crate::loader_version::LoaderVersionId;
use crate::{
    descriptors::{BABRIC_DRIVER, FABRIC_DRIVER, LEGACY_FABRIC_DRIVER},
    driver::DriverDescriptor,
    drivers::fabric::source::FabricFlavor,
};

use self::common::FlavorBehavior;
use self::version::{
    is_legacy_fabric_game_version, is_modern_fabric_game_version, metadata_has_babric_markers,
};

const FABRIC_META_ORIGIN: &str = "https://meta.fabricmc.net";
const FABRIC_MAVEN_ORIGIN: &str = "https://maven.fabricmc.net";
const LEGACY_FABRIC_META_ORIGIN: &str = "https://meta.legacyfabric.net";
const LEGACY_FABRIC_MAVEN_ORIGIN: &str = "https://maven.legacyfabric.net";
const BABRIC_META_ORIGIN: &str = "https://meta.babric.glass-launcher.net";
const BABRIC_MAVEN_ORIGIN: &str = "https://maven.glass-launcher.net/babric";

const FABRIC_LOADER_PREFIXES: LibraryPrefixSet =
    LibraryPrefixSet::new(&["net.fabricmc:fabric-loader:"]);
const LEGACY_FABRIC_LOADER_PREFIXES: LibraryPrefixSet = LibraryPrefixSet::new(&[
    "net.fabricmc:fabric-loader:",
    "net.legacyfabric:fabric-loader:",
]);
const BABRIC_LOADER_PREFIXES: LibraryPrefixSet =
    LibraryPrefixSet::new(&["net.fabricmc:fabric-loader:"]);

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
    loader_prefixes: LibraryPrefixSet,
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

        self.loader_prefixes.version(metadata)
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
        let has_loader = self.loader_prefixes.matches(metadata);
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
