use std::{future::Future, path::Path};

use crate::runtime::distribution::Distribution;

pub mod discovery;
pub mod distribution;
pub mod error;
pub mod provider;
pub mod providers;
mod resolver;

pub use error::{Error, Result};
pub use resolver::RuntimeResolver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeValidationMode {
    Strict,
    Disabled,
}

/// Resolve one runtime with the default provider list.
///
/// This creates a fresh resolver and therefore performs a fresh discovery.
/// Reuse a [`RuntimeResolver`] when resolving multiple runtimes.
pub async fn resolve_runtime(
    required_major_version: usize,
    runtime_executable_path: Option<&Path>,
    validation_mode: RuntimeValidationMode,
    usage: &str,
) -> Result<Distribution> {
    RuntimeResolver::with_default_providers()
        .resolve(
            required_major_version,
            runtime_executable_path,
            validation_mode,
            usage,
        )
        .await
}

pub(crate) async fn resolve_runtime_with<F>(
    required_major_version: usize,
    runtime_executable_path: Option<&Path>,
    validation_mode: RuntimeValidationMode,
    usage: &str,
    cached_distribution: F,
) -> Result<Distribution>
where
    F: Future<Output = Option<Distribution>>,
{
    if let Some(runtime_executable_path) = runtime_executable_path {
        let distribution =
            Distribution::build_from_executable(runtime_executable_path.to_path_buf(), "Launch")
                .await
                .map_err(|source| Error::ResolveRuntime {
                    usage: usage.to_string(),
                    path: runtime_executable_path.to_path_buf(),
                    source: Box::new(source),
                })?;
        let actual_major_version =
            distribution
                .java_major_version()
                .ok_or_else(|| Error::RuntimeVersionUnknown {
                    usage: usage.to_string(),
                    path: runtime_executable_path.to_path_buf(),
                })?;

        if validation_mode == RuntimeValidationMode::Strict
            && actual_major_version != required_major_version
        {
            return Err(Error::VersionMismatch {
                usage: usage.to_string(),
                actual: actual_major_version,
                expected: required_major_version,
                path: runtime_executable_path.to_path_buf(),
            });
        }

        return Ok(distribution);
    }

    cached_distribution
        .await
        .ok_or_else(|| Error::RuntimeNotFound {
            major_version: required_major_version,
            usage: usage.to_string(),
        })
}
