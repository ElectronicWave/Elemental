use std::{fmt, path::PathBuf, sync::Arc};

use elemental_core::runtime::{
    RuntimeResolver,
    provider::{RuntimeProviderList, default_providers},
};
use elemental_driver::families::version_json::{
    BaseInstanceLayout, BaseRootLayout, VersionJsonInstanceLayout, VersionJsonRootLayout,
};
use elemental_infra::downloader::ElementalDownloader;

use crate::{error::Result, launcher::Launcher, origin::DriverOrigins};

pub struct LauncherBuilder<L = BaseRootLayout, VL = BaseInstanceLayout> {
    storage_root: PathBuf,
    downloader: Arc<ElementalDownloader>,
    root_layout: L,
    instance_layout: VL,
    origins: DriverOrigins,
    runtime_providers: RuntimeProviderList,
}

impl<L, VL> fmt::Debug for LauncherBuilder<L, VL>
where
    L: fmt::Debug,
    VL: fmt::Debug,
{
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

impl Default for LauncherBuilder<BaseRootLayout, BaseInstanceLayout> {
    fn default() -> Self {
        Self {
            storage_root: PathBuf::from(".minecraft"),
            downloader: ElementalDownloader::new(),
            root_layout: BaseRootLayout,
            instance_layout: BaseInstanceLayout,
            origins: DriverOrigins::default(),
            runtime_providers: default_providers(),
        }
    }
}

impl LauncherBuilder<BaseRootLayout, BaseInstanceLayout> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<L, VL> LauncherBuilder<L, VL> {
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

    pub fn root_layout<NL>(self, root_layout: NL) -> LauncherBuilder<NL, VL>
    where
        NL: VersionJsonRootLayout + Clone,
    {
        LauncherBuilder {
            storage_root: self.storage_root,
            downloader: self.downloader,
            root_layout,
            instance_layout: self.instance_layout,
            origins: self.origins,
            runtime_providers: self.runtime_providers,
        }
    }

    pub fn instance_layout<NVL>(self, instance_layout: NVL) -> LauncherBuilder<L, NVL>
    where
        NVL: VersionJsonInstanceLayout + Clone + Send,
    {
        LauncherBuilder {
            storage_root: self.storage_root,
            downloader: self.downloader,
            root_layout: self.root_layout,
            instance_layout,
            origins: self.origins,
            runtime_providers: self.runtime_providers,
        }
    }
}

impl<L, VL> LauncherBuilder<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone + Send,
{
    pub fn build(self) -> Launcher<L, VL> {
        Launcher::new(
            self.storage_root,
            self.downloader,
            self.root_layout,
            self.instance_layout,
            self.origins,
            RuntimeResolver::new(self.runtime_providers),
        )
    }
}
