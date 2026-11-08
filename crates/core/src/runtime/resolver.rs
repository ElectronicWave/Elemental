use std::{path::Path, sync::Arc};

use tokio::sync::OnceCell;

use crate::runtime::{
    RuntimeValidationMode,
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

    pub async fn resolve(
        &self,
        required_major_version: usize,
        runtime_executable_path: Option<&Path>,
        validation_mode: RuntimeValidationMode,
    ) -> Result<Distribution> {
        if let Some(runtime_executable_path) = runtime_executable_path {
            let distribution = Distribution::build_from_executable(
                runtime_executable_path.to_path_buf(),
                "Launch",
            )
            .await
            .map_err(|source| Error::ResolveRuntime {
                path: runtime_executable_path.to_path_buf(),
                source: Box::new(source),
            })?;
            let actual_major_version =
                distribution
                    .java_major_version()
                    .ok_or_else(|| Error::RuntimeVersionUnknown {
                        path: runtime_executable_path.to_path_buf(),
                    })?;

            if validation_mode == RuntimeValidationMode::Strict
                && actual_major_version != required_major_version
            {
                return Err(Error::VersionMismatch {
                    actual: actual_major_version,
                    expected: required_major_version,
                    path: runtime_executable_path.to_path_buf(),
                });
            }

            return Ok(distribution);
        }

        self.find_by_java_major(required_major_version)
            .await
            .ok_or(Error::RuntimeNotFound {
                major_version: required_major_version,
            })
    }
}
