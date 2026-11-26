use std::{fmt, path::PathBuf};

use elemental_core::{
    http::{HttpClientConfig, SwappableClient},
    runtime::{RuntimeResolver, provider::RuntimeProviderList, providers::default_providers},
};
use elemental_driver::families::version_json::{
    SharedInstanceLayout, SharedRootLayout, VersionJsonInstanceLayout, VersionJsonRootLayout,
};
use elemental_infra::downloader::ElementalDownloader;

use crate::{
    error::Result, launcher::Launcher, origin::DriverOrigins, registration::DriverRegistration,
};

impl Launcher {
    pub fn builder() -> LauncherBuilder {
        LauncherBuilder::new()
    }
}

pub struct LauncherBuilder {
    storage_root: PathBuf,
    http_client: Option<SwappableClient>,
    downloader: Option<ElementalDownloader>,
    root_layout: SharedRootLayout,
    instance_layout: SharedInstanceLayout,
    origins: DriverOrigins,
    runtime_providers: RuntimeProviderList,
}

impl fmt::Debug for LauncherBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LauncherBuilder")
            .field("storage_root", &self.storage_root)
            .field("http_client", &self.http_client)
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
            http_client: None,
            downloader: None,
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

    /// Sets the shared [`SwappableClient`] distributed to built-in drivers,
    /// catalogs, and the default downloader.
    pub fn http_client(mut self, http_client: SwappableClient) -> Self {
        self.http_client = Some(http_client);
        self
    }

    /// Builds a [`SwappableClient`] from `config` and attaches it to this builder.
    pub fn http_client_config(self, config: HttpClientConfig) -> Result<Self> {
        Ok(self.http_client(SwappableClient::new(config)?))
    }

    pub fn downloader(mut self, downloader: ElementalDownloader) -> Self {
        self.downloader = Some(downloader);
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
        let http_client = self.http_client.unwrap_or_else(|| {
            SwappableClient::try_default().expect("default HTTP client configuration must be valid")
        });
        let downloader = self.downloader.unwrap_or_else(|| {
            ElementalDownloader::builder()
                .http_client(http_client.clone())
                .build()
                .expect("default downloader builder must be valid")
        });

        Launcher::new(
            self.storage_root,
            http_client,
            downloader,
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
