use std::{fmt, path::PathBuf, sync::Arc};

use elemental_core::runtime::{
    RuntimeResolver,
    provider::{RuntimeProviderList, default_providers},
};
use elemental_driver::families::version_json::{
    SharedInstanceLayout, SharedRootLayout, VersionJsonInstanceLayout, VersionJsonRootLayout,
};
use elemental_infra::downloader::ElementalDownloader;

use crate::{
    error::Result, launcher::Launcher, origin::DriverOrigins, registration::DriverRegistration,
};

pub struct LauncherBuilder {
    storage_root: PathBuf,
    downloader: Arc<ElementalDownloader>,
    root_layout: SharedRootLayout,
    instance_layout: SharedInstanceLayout,
    origins: DriverOrigins,
    runtime_providers: RuntimeProviderList,
}

impl fmt::Debug for LauncherBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LauncherBuilder")
            .field("storage_root", &self.storage_root)
            .field("downloader", &self.downloader)
            .field("root_layout", &self.root_layout)
            .field("instance_layout", &self.instance_layout)
            .field("origins", &self.origins)
            .field("runtime_providers", &self.runtime_providers.len())
            .finish()
    }
}

impl Default for LauncherBuilder {
    fn default() -> Self {
        Self {
            storage_root: PathBuf::from(".minecraft"),
            downloader: ElementalDownloader::new(),
            root_layout: SharedRootLayout::default(),
            instance_layout: SharedInstanceLayout::default(),
            origins: DriverOrigins::default(),
            runtime_providers: default_providers(),
        }
    }
}

impl LauncherBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn storage_root(mut self, storage_root: impl Into<PathBuf>) -> Self {
        self.storage_root = storage_root.into();
        self
    }

    pub fn downloader(mut self, downloader: Arc<ElementalDownloader>) -> Self {
        self.downloader = downloader;
        self
    }

    pub fn runtime_providers(mut self, runtime_providers: RuntimeProviderList) -> Self {
        self.runtime_providers = runtime_providers;
        self
    }

    pub fn origin_overrides<F>(mut self, configure: F) -> Result<Self>
    where
        F: FnOnce(&mut DriverOrigins) -> Result<()>,
    {
        configure(&mut self.origins)?;
        Ok(self)
    }

    pub fn root_layout<L>(mut self, root_layout: L) -> Self
    where
        L: VersionJsonRootLayout + 'static,
    {
        self.root_layout = SharedRootLayout::new(root_layout);
        self
    }

    pub fn instance_layout<VL>(mut self, instance_layout: VL) -> Self
    where
        VL: VersionJsonInstanceLayout + 'static,
    {
        self.instance_layout = SharedInstanceLayout::new(instance_layout);
        self
    }

    pub fn build(self) -> Launcher {
        Launcher::new(
            self.storage_root,
            self.downloader,
            self.root_layout,
            self.instance_layout,
            self.origins,
            RuntimeResolver::new(self.runtime_providers),
        )
    }

    /// Builds a launcher and registers third-party drivers.
    pub fn build_with_drivers(self, registrations: Vec<DriverRegistration>) -> Result<Launcher> {
        registrations
            .into_iter()
            .try_fold(self.build(), |launcher, registration| {
                launcher.register_driver(registration)
            })
    }
}
