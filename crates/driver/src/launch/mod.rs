use elemental_core::{
    auth::credential::UserCredential,
    launcher::command::LaunchCommand,
    runtime::{distribution::Distribution, resolve_runtime},
};

use crate::{
    error::Result,
    families::version_json::{
        PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
        builder::VersionJsonLaunchBuilder,
    },
};

mod options;

pub use options::{LaunchOptions, LaunchResolution, QuickPlayOptions, RuntimeOptions};

pub async fn resolve_prepared_version_runtime<L, VL>(
    prepared_version: &PreparedVersionJsonInstance<L, VL>,
    runtime_options: &RuntimeOptions,
) -> Result<Distribution>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let required_major_version = runtime_options
        .runtime_major_version
        .unwrap_or_else(|| prepared_version.required_java_major_version());

    Ok(resolve_runtime(
        required_major_version,
        runtime_options.runtime_executable_path.as_deref(),
        runtime_options.runtime_validation,
        "launch",
    )
    .await?)
}

pub fn launch_builder<L, VL>(
    credential: &UserCredential,
    runtime: Distribution,
    prepared_version: &PreparedVersionJsonInstance<L, VL>,
    options: &LaunchOptions,
) -> Result<VersionJsonLaunchBuilder<L, VL>>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let mut builder = VersionJsonLaunchBuilder::new(
        credential.clone(),
        runtime,
        prepared_version.version.clone(),
    );

    if let Some(client_id) = &options.client_id {
        builder = builder.set_client_id(client_id.clone());
    }

    if let Some(resolution) = &options.resolution {
        builder = builder.set_resolution(resolution.width.clone(), resolution.height.clone());
    }

    if let (Some(name), Some(version)) = (&options.launcher_name, &options.launcher_version) {
        builder = builder.set_launcher(name.clone(), version.clone());
    }

    if let Some(quick_play) = &options.quick_play {
        builder = builder.set_quick_play(
            quick_play.path.clone(),
            quick_play.multiplayer.clone(),
            quick_play.singleplayer.clone(),
            quick_play.realms.clone(),
        );
    }

    if !options.extra_jvm_arguments.is_empty() {
        builder = builder.set_extra_jvm_arguments(options.extra_jvm_arguments.clone());
    }

    if !options.extra_game_arguments.is_empty() {
        builder = builder.set_extra_game_arguments(options.extra_game_arguments.clone());
    }

    Ok(builder)
}

pub async fn build_launch_command<L, VL>(
    credential: &UserCredential,
    prepared_version: &PreparedVersionJsonInstance<L, VL>,
    runtime_options: &RuntimeOptions,
    options: &LaunchOptions,
) -> Result<LaunchCommand>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let runtime = resolve_prepared_version_runtime(prepared_version, runtime_options).await?;
    launch_builder(credential, runtime, prepared_version, options)?.build_command()
}
