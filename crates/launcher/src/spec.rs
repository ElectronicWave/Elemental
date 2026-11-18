use elemental_core::minecraft::MinecraftVersionId;
use elemental_driver::{
    descriptors::{ALL_DRIVERS, VANILLA_DRIVER},
    driver::DriverDescriptor,
    loader_version::LoaderVersionId,
};

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverSpec {
    descriptor: DriverDescriptor,
    game_version: MinecraftVersionId,
    loader_version: Option<LoaderVersionId>,
}

impl DriverSpec {
    pub fn vanilla(game_version: MinecraftVersionId) -> Self {
        Self {
            descriptor: VANILLA_DRIVER,
            game_version,
            loader_version: None,
        }
    }

    pub fn from_descriptor(
        descriptor: DriverDescriptor,
        game_version: MinecraftVersionId,
        loader_version: Option<LoaderVersionId>,
    ) -> Result<Self> {
        if !ALL_DRIVERS.contains(&descriptor) {
            return Err(Error::UnsupportedDriver {
                id: descriptor.id.to_owned(),
            });
        }

        if descriptor == VANILLA_DRIVER {
            if loader_version.is_some() {
                return Err(Error::UnexpectedLoaderVersion {
                    id: descriptor.id.to_owned(),
                });
            }
            return Ok(Self::vanilla(game_version));
        }

        let loader_version = loader_version.ok_or_else(|| Error::LoaderVersionRequired {
            id: descriptor.id.to_owned(),
        })?;

        Ok(Self {
            descriptor,
            game_version,
            loader_version: Some(loader_version),
        })
    }

    pub fn any(
        descriptor: DriverDescriptor,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<Self> {
        Self::from_descriptor(descriptor, game_version, Some(loader_version))
    }

    /// Creates a request for a third-party driver registered on a [`crate::Launcher`].
    ///
    /// Built-in callers should prefer [`DriverSpec::from_descriptor`], which
    /// validates the descriptor and its loader-version requirements eagerly.
    pub fn custom(
        descriptor: DriverDescriptor,
        game_version: MinecraftVersionId,
        loader_version: Option<LoaderVersionId>,
    ) -> Self {
        Self {
            descriptor,
            game_version,
            loader_version,
        }
    }

    pub fn descriptor(&self) -> DriverDescriptor {
        self.descriptor
    }

    pub fn name(&self) -> &'static str {
        self.descriptor.name
    }

    pub fn game_version(&self) -> &MinecraftVersionId {
        &self.game_version
    }

    pub fn loader_version(&self) -> Option<&LoaderVersionId> {
        self.loader_version.as_ref()
    }
}
