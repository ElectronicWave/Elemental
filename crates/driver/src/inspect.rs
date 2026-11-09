mod identity;
mod installed;
mod probe;
mod profile_id;

pub use identity::ProfiledDriverIdentity;
pub use installed::InstalledInstance;
pub use probe::InstanceProbe;
pub use profile_id::ProfileIdPattern;

use elemental_schema::mojang::piston::PistonMetaData;

use crate::driver::{DriverDescriptor, InstalledDriver};
use crate::maven::MavenCoordinate;

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

    fn matches_name(self, library_name: &str) -> bool {
        self.prefixes
            .iter()
            .any(|prefix| library_name.starts_with(prefix))
    }
}
