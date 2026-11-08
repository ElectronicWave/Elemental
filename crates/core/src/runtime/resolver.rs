use std::{path::Path, sync::Arc};

use tokio::sync::OnceCell;

use crate::runtime::{
    RuntimeValidationMode,
    distribution::Distribution,
    error::Result,
    provider::{RuntimeProvider, RuntimeProviderList, default_providers},
    resolve_runtime_with,
};

#[derive(Clone)]
pub struct RuntimeResolver {
    providers: RuntimeProviderList,
    cache: Arc<OnceCell<Vec<Distribution>>>,
}

impl std::fmt::Debug for RuntimeResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeResolver")
            .field("providers", &self.providers.len())
            .finish_non_exhaustive()
    }
}

impl RuntimeResolver {
    pub fn new(providers: RuntimeProviderList) -> Self {
        Self {
            providers,
            cache: Arc::new(OnceCell::new()),
        }
    }

    pub fn with_default_providers() -> Self {
        Self::new(default_providers())
    }

    pub fn providers(&self) -> &[Arc<dyn RuntimeProvider>] {
        &self.providers
    }

    pub async fn distributions(&self) -> Vec<Distribution> {
        self.cache
            .get_or_init(|| async {
                Distribution::from_providers::<Vec<_>>(self.providers.clone()).await
            })
            .await
            .clone()
    }

    pub async fn find_by_java_major(&self, major_version: usize) -> Option<Distribution> {
        self.distributions()
            .await
            .into_iter()
            .find(|distribution| distribution.matches_java_major_version(major_version))
    }

    pub async fn resolve(
        &self,
        required_major_version: usize,
        runtime_executable_path: Option<&Path>,
        validation_mode: RuntimeValidationMode,
        usage: &str,
    ) -> Result<Distribution> {
        resolve_runtime_with(
            required_major_version,
            runtime_executable_path,
            validation_mode,
            usage,
            self.find_by_java_major(required_major_version),
        )
        .await
    }
}

impl Default for RuntimeResolver {
    fn default() -> Self {
        Self::with_default_providers()
    }
}
