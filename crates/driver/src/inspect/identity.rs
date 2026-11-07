use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::driver::{DriverDescriptor, InstalledDriver};

use super::{LibraryPrefixSet, ProfileIdPattern};

/// Identity of a driver whose installed loader version can be recovered from
/// version JSON metadata.
///
/// To support a new driver:
/// - Set `descriptor` to the driver's `DriverDescriptor`.
/// - Put Maven coordinates that only appear when the loader is installed in
///   `markers`. If the loader leaves no such library behind, matching falls back
///   to the profile id shape.
/// - Put the coordinates that carry the installed loader version in
///   `version_libraries`. Coordinates that merely prove the loader is present
///   must stay in `markers`, not here.
/// - Describe the profile id with `ProfileIdPattern`: use `ProfileIdPattern::new`
///   for `<game><separator><loader>` ids and `ProfileIdPattern::loader_prefixed`
///   for `<prefix><loader>` ids.
/// - Add `with_stale_markers` when a legacy coordinate must force a metadata
///   refresh.
///
/// Version lookup prefers `version_libraries` and falls back to the profile id.
/// The shared installer/version-json inspection path consumes this identity
/// directly, so a driver does not need to override `inspect_installed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfiledDriverIdentity {
    descriptor: DriverDescriptor,
    markers: LibraryPrefixSet,
    version_libraries: LibraryPrefixSet,
    profile_id: ProfileIdPattern,
    stale_markers: Option<LibraryPrefixSet>,
}

impl ProfiledDriverIdentity {
    pub const fn new(
        descriptor: DriverDescriptor,
        markers: LibraryPrefixSet,
        version_libraries: LibraryPrefixSet,
        profile_id: ProfileIdPattern,
    ) -> Self {
        Self {
            descriptor,
            markers,
            version_libraries,
            profile_id,
            stale_markers: None,
        }
    }

    pub const fn with_stale_markers(self, stale_markers: LibraryPrefixSet) -> Self {
        Self {
            stale_markers: Some(stale_markers),
            ..self
        }
    }

    pub fn descriptor(self) -> DriverDescriptor {
        self.descriptor
    }

    pub fn build_profile_id(self, game_version: &str, loader_version: &str) -> String {
        self.profile_id.build(game_version, loader_version)
    }

    pub fn inspect_driver_version(self, metadata: &PistonMetaData) -> Option<String> {
        self.version_libraries
            .version(metadata)
            .or_else(|| self.profile_id.loader_version(metadata.id.as_str()))
    }

    pub fn matches_metadata(self, metadata: &PistonMetaData) -> bool {
        self.markers.matches(metadata)
            || self
                .profile_id
                .loader_version(metadata.id.as_str())
                .is_some()
    }

    pub fn local_metadata_needs_refresh(
        self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &str,
    ) -> bool {
        metadata.id != self.build_profile_id(game_version.as_str(), loader_version)
            || metadata.inherits_from.as_deref() != Some(game_version.as_str())
            || self
                .stale_markers
                .is_some_and(|stale_markers| stale_markers.matches(metadata))
            || self
                .inspect_driver_version(metadata)
                .is_none_or(|installed| installed != loader_version)
    }

    pub fn inspect_installed(self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        if !self.matches_metadata(metadata) {
            return None;
        }

        Some(InstalledDriver::version_json(
            self.descriptor,
            metadata,
            self.inspect_driver_version(metadata),
        ))
    }
}
