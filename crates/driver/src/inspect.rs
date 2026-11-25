mod identity;
mod profile_id;

pub use identity::ProfiledDriverIdentity;
pub use profile_id::ProfileIdPattern;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::descriptors::DriverDescriptor;
use crate::maven::MavenCoordinate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledDriver {
    pub driver: DriverDescriptor,
    pub driver_version: Option<String>,
    pub game_version: Option<MinecraftVersionId>,
    pub description: Option<String>,
}

impl InstalledDriver {
    pub fn version_json(
        descriptor: DriverDescriptor,
        metadata: &PistonMetaData,
        driver_version: Option<String>,
    ) -> Self {
        let game_version = metadata
            .inherits_from
            .clone()
            .unwrap_or_else(|| metadata.id.clone());
        Self {
            driver: descriptor,
            driver_version,
            game_version: Some(MinecraftVersionId::from(game_version)),
            description: Some(metadata.release_type.clone()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibraryPrefixSet {
    prefixes: &'static [&'static str],
}

impl LibraryPrefixSet {
    pub const fn new(prefixes: &'static [&'static str]) -> Self {
        Self { prefixes }
    }

    pub fn matches(self, metadata: &PistonMetaData) -> bool {
        metadata
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .any(|name| self.matches_name(name))
    }

    pub fn version(self, metadata: &PistonMetaData) -> Option<String> {
        metadata
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .find(|name| self.matches_name(name))
            .and_then(|name| {
                MavenCoordinate::parse(name).map(|coordinate| coordinate.version().to_owned())
            })
    }

    pub fn installed_driver(
        self,
        metadata: &PistonMetaData,
        descriptor: DriverDescriptor,
    ) -> Option<InstalledDriver> {
        self.version(metadata).map(|driver_version| {
            InstalledDriver::version_json(descriptor, metadata, Some(driver_version))
        })
    }

    pub(crate) fn matches_name(self, library_name: &str) -> bool {
        self.prefixes
            .iter()
            .any(|prefix| library_name.starts_with(prefix))
    }
}
