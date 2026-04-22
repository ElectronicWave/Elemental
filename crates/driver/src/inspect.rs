use std::{fs::File, path::PathBuf};

use anyhow::Result;
use elemental_core::{
    minecraft::MinecraftVersionId,
    storage::{
        Storage,
        layout::{Layout, Layoutable},
    },
};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    families::version_json::{
        VersionJsonGameStorageExt, VersionJsonInstanceLayout, VersionJsonInstanceResource,
        VersionJsonRootLayout,
    },
};

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
            .and_then(|name| name.split(':').nth(2).map(ToOwned::to_owned))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileIdPattern {
    separator: &'static str,
}

impl ProfileIdPattern {
    pub const fn new(separator: &'static str) -> Self {
        Self { separator }
    }

    pub fn build(self, game_version: &str, loader_version: &str) -> String {
        format!("{game_version}{}{loader_version}", self.separator)
    }

    pub fn loader_version(self, metadata_id: &str) -> Option<String> {
        metadata_id
            .split_once(self.separator)
            .map(|(_, loader_version)| loader_version.to_owned())
    }
}

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

#[derive(Debug, Clone)]
pub struct InstanceProbe<L: Layout, VL: Layout> {
    pub storage: Storage<VL, Storage<L>>,
    pub metadata: Option<PistonMetaData>,
}

#[derive(Debug, Clone)]
pub struct InstalledInstance<L: Layout, VL: Layout> {
    pub storage: Storage<VL, Storage<L>>,
    pub driver: InstalledDriver,
}

impl<L: Layout, VL: VersionJsonInstanceLayout> InstanceProbe<L, VL> {
    pub fn collect(storage: Storage<VL, Storage<L>>) -> Result<Self> {
        let metadata_path = Self::detect_metadata_path(&storage)?;
        let metadata: Option<PistonMetaData> = match metadata_path.as_ref() {
            Some(path) => Some(serde_json::from_reader(File::open(path)?)?),
            None => None,
        };

        Ok(Self { storage, metadata })
    }

    fn detect_metadata_path(storage: &Storage<VL, Storage<L>>) -> Result<Option<PathBuf>> {
        let preferred = storage
            .try_get_resource(VersionJsonInstanceResource::Metadata)
            .ok()
            .filter(|path| path.exists());
        if preferred.is_some() {
            return Ok(preferred);
        }

        let mut json_files = Vec::new();
        if storage.path.exists() {
            for entry in storage.path.read_dir()? {
                let entry = entry?;
                if entry.file_type()?.is_file()
                    && entry
                        .path()
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
                {
                    json_files.push(entry.path());
                }
            }
        }

        Ok(if json_files.len() == 1 {
            json_files.into_iter().next()
        } else {
            None
        })
    }
}

impl<L: Layout + Clone, VL: VersionJsonInstanceLayout + Clone> InstalledInstance<L, VL> {
    pub async fn detect(
        storage: Storage<VL, Storage<L>>,
        drivers: &[&dyn Driver<L, VL>],
    ) -> Result<Option<Self>> {
        let probe = InstanceProbe::collect(storage.clone())?;
        for driver in drivers {
            if let Some(installed) = driver.inspect(&probe).await? {
                return Ok(Some(Self {
                    storage,
                    driver: installed,
                }));
            }
        }

        Ok(None)
    }
}

impl<L, VL> InstalledInstance<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    pub async fn detect_all(
        storage: &Storage<L>,
        version_layout: VL,
        drivers: &[&dyn Driver<L, VL>],
    ) -> Result<Vec<Self>> {
        let mut instances = Vec::new();

        for instance in storage.instances(version_layout)? {
            if let Some(installed) = Self::detect(instance, drivers).await? {
                instances.push(installed);
            }
        }

        Ok(instances)
    }
}
