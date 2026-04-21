use std::path::Path;

use anyhow::Result;
use elemental_core::{
    auth::authorizer::Authorizer,
    launcher::{command::LaunchCommand, process},
    runtime::{distribution::Distribution, resolve_runtime},
};

use crate::families::version_json::{
    LaunchedVersionJsonInstance, PreparedVersionJsonInstance, VersionJsonInstanceLayout,
    VersionJsonRemoteResolver, VersionJsonRootLayout, builder::VersionJsonLaunchBuilder,
};

pub mod arguments;
pub mod config;

pub use arguments::parse_argument_string;
pub use config::{LaunchResolution, QuickPlayOptions, VersionJsonLaunchConfig};

pub async fn resolve_prepared_version_runtime<RR, L, VL>(
    prepared_version: &PreparedVersionJsonInstance<RR, L, VL>,
    runtime_major_version: Option<usize>,
    runtime_executable_path: Option<&Path>,
) -> Result<Distribution>
where
    RR: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let required_major_version =
        runtime_major_version.unwrap_or_else(|| prepared_version.required_java_major_version());

    resolve_runtime(required_major_version, runtime_executable_path, "launch").await
}

pub fn build_version_json_launch_builder<A, RR, L, VL>(
    authorizer: A,
    runtime: Distribution,
    prepared_version: &PreparedVersionJsonInstance<RR, L, VL>,
    config: &VersionJsonLaunchConfig,
) -> Result<VersionJsonLaunchBuilder<A, L, VL>>
where
    A: Authorizer,
    RR: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let mut builder = VersionJsonLaunchBuilder::new(
        authorizer,
        runtime,
        prepared_version.resolved_version.version.clone(),
    );

    if let Some(client_id) = &config.client_id {
        builder = builder.set_client_id(client_id.clone());
    }

    if let Some(resolution) = &config.resolution {
        builder = builder.set_resolution(resolution.width.clone(), resolution.height.clone());
    }

    if let (Some(name), Some(version)) = (&config.launcher_name, &config.launcher_version) {
        builder = builder.set_launcher(name.clone(), version.clone());
    }

    if let Some(quick_play) = &config.quick_play {
        builder = builder.set_quick_play(
            quick_play.path.clone(),
            quick_play.multiplayer.clone(),
            quick_play.singleplayer.clone(),
            quick_play.realms.clone(),
        );
    }

    if !config.extra_jvm_arguments.is_empty() {
        builder = builder.set_extra_jvm_arguments(config.extra_jvm_arguments.clone());
    }

    if !config.extra_game_arguments.is_empty() {
        builder = builder.set_extra_game_arguments(config.extra_game_arguments.clone());
    }

    Ok(builder)
}

pub async fn build_version_json_launch_command<A, R, L, VL>(
    authorizer: A,
    prepared_version: &PreparedVersionJsonInstance<R, L, VL>,
    config: &VersionJsonLaunchConfig,
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
    config: &VersionJsonLaunchConfig,
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
    config: &VersionJsonLaunchConfig,
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
