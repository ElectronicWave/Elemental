use std::str::FromStr;

use async_trait::async_trait;
use elemental_core::{
    minecraft::MinecraftVersionId,
    runtime::RuntimeResolver,
    storage::{Storage, layout::Layout},
};
use elemental_infra::downloader::DownloadSession;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::descriptors::from_id;
use crate::error::{Error, Result};
use crate::families::version_json::{
    PistonMetaDataExt, PreparedVersionJsonInstance, VersionJsonInstanceLayout,
    VersionJsonRootLayout,
};
use crate::inspect::InstanceProbe;
use crate::launch::RuntimeOptions;
use crate::loader_version::LoaderVersionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverDescriptor {
    pub id: &'static str,
    pub name: &'static str,
}

impl TryFrom<&str> for DriverDescriptor {
    type Error = Error;

    fn try_from(id: &str) -> Result<Self> {
        from_id(id).ok_or_else(|| Error::UnsupportedDriver { id: id.to_owned() })
    }
}

impl TryFrom<String> for DriverDescriptor {
    type Error = Error;

    fn try_from(id: String) -> Result<Self> {
        Self::try_from(id.as_str())
    }
}

impl FromStr for DriverDescriptor {
    type Err = Error;

    fn from_str(id: &str) -> Result<Self> {
        Self::try_from(id)
    }
}

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
        Self {
            driver: descriptor,
            driver_version,
            game_version: Some(MinecraftVersionId::from(metadata.game_version())),
            description: Some(metadata.release_type.clone()),
        }
    }
}

pub struct PrepareRequest<'a, L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub session: &'a DownloadSession,
    pub instance: &'a Storage<VL, Storage<L>>,
    pub game_version: &'a MinecraftVersionId,
    pub loader_version: Option<&'a LoaderVersionId>,
    pub runtime_options: &'a RuntimeOptions,
    pub runtime_resolver: &'a RuntimeResolver,
}

impl<'a, L, VL> PrepareRequest<'a, L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn required_loader_version(&self, driver: DriverDescriptor) -> Result<&'a LoaderVersionId> {
        self.loader_version
            .ok_or_else(|| Error::LoaderVersionRequired {
                driver: driver.id.to_owned(),
            })
    }
}

#[async_trait(?Send)]
pub trait Driver<L: Layout, VL: Layout> {
    fn descriptor(&self) -> DriverDescriptor;

    async fn inspect(&self, probe: &InstanceProbe) -> Result<Option<InstalledDriver>>;

    /// Prepare a version for launch and return the common version JSON view.
    ///
    /// The result contains everything launch construction needs; family-specific
    /// installation state is not part of the public result.
    async fn prepare(
        &self,
        request: &PrepareRequest<'_, L, VL>,
    ) -> Result<PreparedVersionJsonInstance<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone;

    /// Load a prepared version and return the common version JSON view.
    ///
    /// The result contains everything launch construction needs; family-specific
    /// installation state is not part of the public result.
    async fn load_prepared(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedVersionJsonInstance<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone;
}
