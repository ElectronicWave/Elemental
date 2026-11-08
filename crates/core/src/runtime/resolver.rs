use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::runtime::{
    distribution::Distribution,
    error::{Error, Result},
    provider::{RuntimeProviderList, default_providers},
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

    pub async fn distributions(&self) -> Vec<Distribution> {
        self.cache
            .get_or_init(|| async {
                Distribution::from_providers::<Vec<_>>(self.providers.clone()).await
            })
            .await
            .clone()
    }

    async fn find_by_java_major(&self, major_version: usize) -> Option<Distribution> {
        self.distributions()
            .await
            .into_iter()
            .find(|distribution| distribution.matches_java_major_version(major_version))
    }

    pub async fn resolve(&self, required_major_version: usize) -> Result<Distribution> {
        self.find_by_java_major(required_major_version)
            .await
            .ok_or(Error::RuntimeNotFound {
                major_version: required_major_version,
            })
    }
}
