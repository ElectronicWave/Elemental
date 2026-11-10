use elemental_core::runtime::{RuntimeResolver, distribution::Distribution};

use crate::{
    error::Result,
    families::version_json::{
        PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
    },
};

mod options;

pub use options::RuntimeOptions;

pub async fn resolve_prepared_version_runtime<L, VL>(
    prepared_version: &PreparedVersionJsonInstance<L, VL>,
    runtime_options: &RuntimeOptions,
    runtime_resolver: &RuntimeResolver,
) -> Result<Distribution>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let required_major_version = runtime_options
        .runtime_major_version
        .unwrap_or_else(|| prepared_version.required_java_major_version());

    if let Some(distribution) = runtime_options.runtime_override.as_ref() {
        distribution
            .require_compatible(required_major_version, runtime_options.runtime_validation)?;
        return Ok(distribution.clone());
    }

    Ok(runtime_resolver.resolve(required_major_version).await?)
}
