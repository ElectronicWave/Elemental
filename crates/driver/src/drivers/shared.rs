use anyhow::{Context, Result};
use elemental_core::{
    auth::authorizer::Authorizer,
    launcher::{command::LaunchCommand, process},
    runtime::distribution::Distribution,
    storage::Storage,
};

use crate::{
    driver::{DriverDescriptor, InstalledDriver},
    drivers::vanilla::{
        config::VanillaLaunchConfig,
        prepared::ResolvedVanillaMetadata,
        source::{VanillaEndpoints, VanillaSource},
    },
    families::version_json::{
        LaunchedVersionJsonInstance, PistonMetaData, PreparedVersionJsonInstance,
        ResolvedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRemoteResolver,
        VersionJsonRootLayout,
    },
    launch::{build_version_json_launch_builder, resolve_prepared_version_runtime},
};

pub async fn load_prepared_version_json<R, L, VL>(
    remote_resolver: R,
    instance: &Storage<VL, Storage<L>>,
) -> Result<PreparedVersionJsonInstance<R, L, VL>>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    ResolvedVersionJsonInstance::load(remote_resolver, instance.clone())?
        .into_prepared()
        .await
}

pub async fn build_version_json_launch_command<A, R, L, VL>(
    authorizer: A,
    prepared_version: &PreparedVersionJsonInstance<R, L, VL>,
    config: &VanillaLaunchConfig,
) -> Result<(Distribution, LaunchCommand)>
where
    A: Authorizer,
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let runtime = resolve_prepared_version_runtime(
        prepared_version,
        config.runtime_major_version,
        config.runtime_executable_path.as_deref(),
    )
    .await?;
    let command =
        build_version_json_launch_builder(authorizer, runtime.clone(), prepared_version, config)?
            .build_command()
            .await?;

    Ok((runtime, command))
}

pub async fn launch_wrapped_version<A, P, AccessFn, WrapFn, Output, R, L, VL>(
    authorizer: A,
    prepared_version: P,
    config: &VanillaLaunchConfig,
    access_launch_version: AccessFn,
    wrap: WrapFn,
) -> Result<Output>
where
    A: Authorizer,
    AccessFn: Fn(&P) -> &PreparedVersionJsonInstance<R, L, VL>,
    WrapFn: FnOnce(P, Distribution, tokio::process::Child) -> Output,
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let (runtime, command) = build_version_json_launch_command(
        authorizer,
        access_launch_version(&prepared_version),
        config,
    )
    .await?;
    let child = process::spawn_command(command)?;

    Ok(wrap(prepared_version, runtime, child))
}

pub async fn launch_version_json_instance<A, R, L, VL>(
    authorizer: A,
    prepared_version: PreparedVersionJsonInstance<R, L, VL>,
    config: &VanillaLaunchConfig,
) -> Result<LaunchedVersionJsonInstance<R, L, VL>>
where
    A: Authorizer,
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    launch_wrapped_version(
        authorizer,
        prepared_version,
        config,
        |prepared_version| prepared_version,
        |prepared_version, runtime, child| LaunchedVersionJsonInstance {
            prepared_version,
            runtime,
            child,
        },
    )
    .await
}

pub async fn resolve_vanilla_metadata(
    vanilla_source: &VanillaSource,
    game_version: &str,
) -> Result<ResolvedVanillaMetadata> {
    let launchmeta = vanilla_source.launch_meta().await?;
    let metadata_url = launchmeta
        .versions
        .iter()
        .find(|version| version.id == game_version)
        .with_context(|| format!("Can't find version named `{game_version}`"))?
        .url
        .clone();
    let metadata = vanilla_source.piston_meta(metadata_url).await?;
    let asset_index_objects = vanilla_source
        .asset_index_objects(&metadata.asset_index.url)
        .await?;

    Ok(ResolvedVanillaMetadata::new(
        vanilla_source.endpoints().clone(),
        metadata,
        asset_index_objects,
    ))
}

pub fn installed_version_json_driver(
    metadata: &PistonMetaData,
    descriptor: DriverDescriptor,
    driver_version: Option<String>,
) -> InstalledDriver {
    InstalledDriver {
        driver: descriptor,
        driver_version,
        game_version: metadata
            .inherits_from
            .clone()
            .or_else(|| Some(metadata.id.clone())),
        description: Some(metadata.release_type.clone()),
    }
}

pub fn metadata_contains_library_prefix(metadata: &PistonMetaData, prefixes: &[&str]) -> bool {
    metadata.libraries.iter().any(|library| {
        let name = library.name.as_str();
        prefixes.iter().any(|prefix| name.starts_with(prefix))
    })
}

pub fn find_library_version(metadata: &PistonMetaData, prefixes: &[&str]) -> Option<String> {
    metadata
        .libraries
        .iter()
        .map(|library| library.name.as_str())
        .find(|name| prefixes.iter().any(|prefix| name.starts_with(prefix)))
        .and_then(|name| name.split(':').nth(2).map(ToOwned::to_owned))
}

pub fn rewrite_upstream_with_vanilla_fallback<RewriteFn>(
    vanilla_endpoints: &VanillaEndpoints,
    raw_url: &str,
    family_name: &str,
    rewrite_family: RewriteFn,
) -> Result<String>
where
    RewriteFn: FnOnce() -> Result<String>,
{
    if let Ok(rewritten) = vanilla_endpoints.rewrite_upstream(raw_url) {
        return Ok(rewritten);
    }

    rewrite_family()
        .with_context(|| format!("rewrite {family_name} upstream url failed for '{raw_url}'"))
}
