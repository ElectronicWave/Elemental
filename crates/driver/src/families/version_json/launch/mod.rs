use std::path::Path;

use crate::error::Result;
use elemental_core::{
    auth::authorizer::Authorizer,
    launcher::command::LaunchCommand,
    runtime::{RuntimeValidationMode, distribution::Distribution, resolve_runtime},
};

use crate::families::version_json::{
    PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
    builder::VersionJsonLaunchBuilder,
};

pub mod arguments;
pub mod config;

pub use arguments::parse_argument_string;
pub use config::{LaunchResolution, QuickPlayOptions, VersionJsonLaunchConfig};

pub async fn resolve_prepared_version_runtime<L, VL>(
    prepared_version: &PreparedVersionJsonInstance<L, VL>,
    runtime_major_version: Option<usize>,
    runtime_executable_path: Option<&Path>,
    runtime_validation: RuntimeValidationMode,
) -> Result<Distribution>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let required_major_version =
        runtime_major_version.unwrap_or_else(|| prepared_version.required_java_major_version());

    Ok(resolve_runtime(
        required_major_version,
        runtime_executable_path,
        runtime_validation,
        "launch",
    )
    .await?)
}

pub fn build_version_json_launch_builder<A, L, VL>(
    authorizer: A,
    runtime: Distribution,
    prepared_version: &PreparedVersionJsonInstance<L, VL>,
    config: &VersionJsonLaunchConfig,
) -> Result<VersionJsonLaunchBuilder<A, L, VL>>
where
    A: Authorizer,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let mut builder =
        VersionJsonLaunchBuilder::new(authorizer, runtime, prepared_version.version.clone());

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

pub async fn build_version_json_launch_command<A, L, VL>(
    authorizer: A,
    prepared_version: &PreparedVersionJsonInstance<L, VL>,
    config: &VersionJsonLaunchConfig,
) -> Result<(Distribution, LaunchCommand)>
where
    A: Authorizer,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let runtime = resolve_prepared_version_runtime(
        prepared_version,
        config.runtime_major_version,
        config.runtime_executable_path.as_deref(),
        config.runtime_validation,
    )
    .await?;
    let command =
        build_version_json_launch_builder(authorizer, runtime.clone(), prepared_version, config)?
            .build_command()
            .await?;

    Ok((runtime, command))
}
