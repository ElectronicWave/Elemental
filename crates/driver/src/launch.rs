use std::path::Path;

use anyhow::Result;
use elemental_core::{auth::authorizer::Authorizer, runtime::distribution::Distribution};

use crate::{
    drivers::vanilla::config::VanillaLaunchConfig,
    families::version_json::{
        PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRemoteResolver,
        VersionJsonRootLayout, builder::VersionJsonLaunchBuilder,
    },
    runtime::resolve_runtime,
};

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
    config: &VanillaLaunchConfig,
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
