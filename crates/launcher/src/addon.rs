use std::{fmt, sync::Arc};

#[cfg(feature = "yggdrasil-auth")]
use elemental_addon::{AUTHLIB_INJECTOR_ADDON_ID, AuthlibInjectorAddon, AuthlibInjectorEndpoints};
use elemental_addon::{Addon, AddonDescriptor, AddonPatchMode};
#[cfg(feature = "addon-liteloader")]
use elemental_addon::{
    LITELOADER_ADDON_ID, LiteLoaderAddon, LiteLoaderEndpoints, LiteLoaderSource,
};
use elemental_core::http::SwappableClient;

use crate::{
    error::{Error, Result},
    origin::DriverOrigins,
};

/// Declarative request selecting a registered addon by ID, optional version, and scoped patch mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonSpec {
    /// Registered addon ID to look up in [`AddonRegistry`].
    pub id: String,
    /// Optional addon version passed to [`Addon::prepare`].
    pub version: Option<String>,
    /// Per-addon patch mode preference (defaults to [`AddonPatchMode::Inherit`]).
    pub patch_mode: AddonPatchMode,
}

impl AddonSpec {
    /// Creates a request targeting the registered addon with `id`.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: None,
            patch_mode: AddonPatchMode::Inherit,
        }
    }

    /// Selects a specific addon release version to install.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Sets the per-addon patch mode preference (`Inherit`, `Virtual`, or `Physical`).
    pub fn with_patch_mode(mut self, patch_mode: AddonPatchMode) -> Self {
        self.patch_mode = patch_mode;
        self
    }
}

/// Built-in and third-party addons registered on a [`crate::Launcher`].
#[derive(Clone)]
pub struct AddonRegistry {
    addons: Vec<Arc<dyn Addon>>,
}

impl fmt::Debug for AddonRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ids = self
            .addons
            .iter()
            .map(|addon| addon.descriptor().id)
            .collect::<Vec<_>>();
        f.debug_struct("AddonRegistry")
            .field("addons", &ids)
            .finish()
    }
}

impl AddonRegistry {
    pub(crate) fn new(http_client: SwappableClient, origins: &DriverOrigins) -> Self {
        let _ = (&http_client, origins);
        let addons: Vec<Arc<dyn Addon>> = vec![
            #[cfg(feature = "addon-liteloader")]
            Arc::new(
                LiteLoaderAddon::new(LiteLoaderSource::new(LiteLoaderEndpoints::new(
                    origins.policy_for_addon(LITELOADER_ADDON_ID),
                )))
                .with_http_client(http_client.clone()),
            ),
            #[cfg(feature = "yggdrasil-auth")]
            Arc::new(
                AuthlibInjectorAddon::new()
                    .with_endpoints(AuthlibInjectorEndpoints::new(
                        origins.policy_for_addon(AUTHLIB_INJECTOR_ADDON_ID),
                    ))
                    .with_http_client(http_client.clone()),
            ),
        ];

        Self { addons }
    }

    pub(crate) fn register(&mut self, addon: Arc<dyn Addon>) -> Result<()> {
        let descriptor = addon.descriptor();
        if self
            .addons
            .iter()
            .any(|registered| registered.descriptor().id == descriptor.id)
        {
            return Err(Error::AddonAlreadyRegistered {
                id: descriptor.id.into_owned(),
            });
        }
        self.addons.push(addon);
        Ok(())
    }

    /// Returns the registered addon matching `id`, if any.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Addon>> {
        self.addons
            .iter()
            .find(|addon| addon.descriptor().id == id)
            .cloned()
    }

    /// Returns the registered addon matching `id`, or [`Error::AddonNotRegistered`] if absent.
    pub fn addon(&self, id: &str) -> Result<Arc<dyn Addon>> {
        self.get(id)
            .ok_or_else(|| Error::AddonNotRegistered { id: id.to_owned() })
    }

    /// Iterates over descriptors of all registered addons.
    pub fn descriptors(&self) -> impl Iterator<Item = AddonDescriptor> + '_ {
        self.addons.iter().map(|addon| addon.descriptor())
    }

    /// Iterates over all registered addon instances.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Addon>> {
        self.addons.iter()
    }
}
