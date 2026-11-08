use async_trait::async_trait;
use std::{path::PathBuf, sync::Arc};

#[async_trait]
pub trait RuntimeProvider: Send + Sync {
    async fn list(&self) -> Vec<PathBuf>;

    #[inline(always)]
    fn name(&self) -> &'static str {
        "Provider"
    }

    fn arc_default() -> Arc<Self>
    where
        Self: Sized + Default,
    {
        Arc::new(Self::default())
    }
}

/// Re-export providers
use super::providers::{
    envjavahome::EnvJavaHomeProvider, envpath::EnvPathProvider, pm::PackageManagerProvider,
    registry::RegistryProvider,
};

pub type RuntimeProviderList = Vec<Arc<dyn RuntimeProvider>>;

pub fn default_providers() -> RuntimeProviderList {
    vec![
        RegistryProvider::arc_default(),
        EnvPathProvider::arc_default(),
        PackageManagerProvider::arc_default(),
        EnvJavaHomeProvider::arc_default(),
    ]
}
