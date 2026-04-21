use async_trait::async_trait;
use std::{
    path::PathBuf,
    sync::{Arc, OnceLock, RwLock},
};

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

static RUNTIME_PROVIDER_OVERRIDE: OnceLock<RwLock<Option<Vec<Arc<dyn RuntimeProvider>>>>> =
    OnceLock::new();

pub fn default_providers() -> Vec<Arc<dyn RuntimeProvider>> {
    vec![
        RegistryProvider::arc_default(),
        EnvPathProvider::arc_default(),
        PackageManagerProvider::arc_default(),
        EnvJavaHomeProvider::arc_default(),
    ]
}

pub fn runtime_providers() -> Vec<Arc<dyn RuntimeProvider>> {
    let storage = RUNTIME_PROVIDER_OVERRIDE.get_or_init(|| RwLock::new(None));
    let Ok(guard) = storage.read() else {
        return default_providers();
    };

    guard.clone().unwrap_or_else(default_providers)
}

pub fn with_runtime_providers(providers: Vec<Arc<dyn RuntimeProvider>>) -> anyhow::Result<()> {
    let storage = RUNTIME_PROVIDER_OVERRIDE.get_or_init(|| RwLock::new(None));
    let mut guard = storage
        .write()
        .map_err(|_| anyhow::anyhow!("lock runtime provider override for writing failed"))?;
    *guard = Some(providers);
    Ok(())
}
