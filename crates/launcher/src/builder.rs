use std::{path::PathBuf, sync::Arc};

use elemental_driver::families::version_json::{
    BaseInstanceLayout, BaseRootLayout, VersionJsonInstanceLayout, VersionJsonRootLayout,
};
use elemental_infra::downloader::core::ElementalDownloader;

use crate::launcher::Launcher;

#[derive(Debug)]
pub struct LauncherBuilder<L = BaseRootLayout, VL = BaseInstanceLayout>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    storage_root: PathBuf,
    downloader: Arc<ElementalDownloader>,
    root_layout: L,
    instance_layout: VL,
}

impl Default for LauncherBuilder<BaseRootLayout, BaseInstanceLayout> {
    fn default() -> Self {
        Self {
            storage_root: PathBuf::from(".minecraft"),
            downloader: ElementalDownloader::new(),
            root_layout: BaseRootLayout,
            instance_layout: BaseInstanceLayout,
        }
    }
}

impl LauncherBuilder<BaseRootLayout, BaseInstanceLayout> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<L, VL> LauncherBuilder<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone + Send,
{
    pub fn with_layouts<NL, NVL>(
        self,
        root_layout: NL,
        instance_layout: NVL,
    ) -> LauncherBuilder<NL, NVL>
    where
        NL: VersionJsonRootLayout + Clone,
        NVL: VersionJsonInstanceLayout + Clone + Send,
    {
        LauncherBuilder {
            storage_root: self.storage_root,
            downloader: self.downloader,
            root_layout,
            instance_layout,
        }
    }

    pub fn storage_root(mut self, storage_root: impl Into<PathBuf>) -> Self {
        self.storage_root = storage_root.into();
        self
    }

    pub fn downloader(mut self, downloader: Arc<ElementalDownloader>) -> Self {
        self.downloader = downloader;
        self
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
        }
    }

    pub fn build(self) -> Launcher<L, VL> {
        Launcher::with_layouts(
            self.storage_root,
            self.downloader,
            self.root_layout,
            self.instance_layout,
        )
    }
}
