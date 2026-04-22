use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::runtime::distribution::Distribution;

pub mod distribution;
pub mod provider;
pub mod providers;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeValidationMode {
    Strict,
    Disabled,
}

pub async fn resolve_runtime(
    required_major_version: usize,
    runtime_executable_path: Option<&Path>,
    validation_mode: RuntimeValidationMode,
    usage: &str,
) -> Result<Distribution> {
    if let Some(runtime_executable_path) = runtime_executable_path {
        let distribution =
            Distribution::build_from_executable(runtime_executable_path.to_path_buf(), "Launch")
                .await
                .with_context(|| {
                    format!(
                        "resolve {} runtime from executable failed: {}",
                        usage,
                        runtime_executable_path.display()
                    )
                })?;
        let actual_major_version = distribution.java_major_version().context(format!(
            "can't determine Java major version for explicit {} runtime executable: {}",
            usage,
            runtime_executable_path.display()
        ))?;

        if validation_mode == RuntimeValidationMode::Strict
            && actual_major_version != required_major_version
        {
            bail!(
                "explicit {} runtime executable has Java major version {}, expected {}: {}",
                usage,
                actual_major_version,
                required_major_version,
                runtime_executable_path.display()
            );
        }

        return Ok(distribution);
    }

    Distribution::find_cached_by_java_major(required_major_version)
        .await
        .with_context(|| {
            format!(
                "can't find a local Java runtime with major version {} for {}",
                required_major_version, usage
            )
        })
}
