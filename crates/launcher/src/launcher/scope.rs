use elemental_addon::{Addon, AddonPatchMode, PatchMode, PatchTrace};
use elemental_core::{auth::credential::Credential, launcher::command::LaunchCommand};
use elemental_driver::driver::PrepareStage;
use elemental_infra::downloader::{DownloadSession, ElementalDownloader};

use crate::{
    addon::AddonSpec,
    error::Result,
    request::{LaunchOptions, RuntimeOptions},
    result::{LaunchedInstance, PreparedInstance},
    spec::DriverSpec,
};

use super::{
    addons::{
        LaunchHookEntry, RequestedAddon, apply_launch_addon_hooks, order_and_trace_launch_entries,
        resolve_launch_addons,
    },
    ops::{self, LauncherServices, PrepareSessionParams},
};

/// Scoped operation context created by [`Launcher::scope`](super::Launcher::scope).
///
/// Configure shared execution parameters (runtime, download session, addons,
/// or persistent patch mode) once, then choose the terminal action to execute.
pub struct ActionScope<'a> {
    services: LauncherServices<'a>,
    downloader: &'a ElementalDownloader,
    runtime_options: Option<&'a RuntimeOptions>,
    session: Option<DownloadSession>,
    addons: Vec<RequestedAddon>,
    patch_mode: Option<PatchMode>,
}

impl<'a> ActionScope<'a> {
    pub(super) fn new(services: LauncherServices<'a>, downloader: &'a ElementalDownloader) -> Self {
        Self {
            services,
            downloader,
            runtime_options: None,
            session: None,
            addons: Vec::new(),
            patch_mode: None,
        }
    }

    pub fn runtime_options(mut self, options: &'a RuntimeOptions) -> Self {
        self.runtime_options = Some(options);
        self
    }

    pub fn session(mut self, session: DownloadSession) -> Self {
        self.session = Some(session);
        self
    }

    pub fn addon(mut self, spec: AddonSpec) -> Self {
        self.addons.push(RequestedAddon::BySpec(spec));
        self
    }

    pub fn with_addon(mut self, addon: impl Addon + 'static, version: Option<String>) -> Self {
        self.addons.push(RequestedAddon::direct(
            addon,
            version,
            AddonPatchMode::Inherit,
        ));
        self
    }

    pub fn with_addon_mode(
        mut self,
        addon: impl Addon + 'static,
        version: Option<String>,
        patch_mode: AddonPatchMode,
    ) -> Self {
        self.addons
            .push(RequestedAddon::direct(addon, version, patch_mode));
        self
    }

    pub fn patch_mode(mut self, mode: PatchMode) -> Self {
        self.patch_mode = Some(mode);
        self
    }

    /// Prepares or refreshes an instance and installs any scoped persistent addons.
    pub async fn prepare_instance(
        self,
        instance_name: impl Into<String>,
        driver: DriverSpec,
    ) -> Result<PreparedInstance> {
        self.run_prepare(instance_name.into(), driver, None).await
    }

    /// Prepares or refreshes an instance while observing stage transitions.
    pub async fn prepare_instance_with_stage(
        self,
        instance_name: impl Into<String>,
        driver: DriverSpec,
        on_stage: impl Fn(PrepareStage) + Send + Sync,
    ) -> Result<PreparedInstance> {
        self.run_prepare(instance_name.into(), driver, Some(&on_stage))
            .await
    }

    async fn run_prepare(
        self,
        instance_name: String,
        driver: DriverSpec,
        on_stage: Option<&(dyn Fn(PrepareStage) + Send + Sync)>,
    ) -> Result<PreparedInstance> {
        let session = open_or_reuse_session(
            self.downloader,
            self.session,
            &format!("prepare {instance_name}"),
        )
        .await?;
        let default_runtime = RuntimeOptions::default();
        let runtime_options = self.runtime_options.unwrap_or(&default_runtime);
        let params = PrepareSessionParams::new(
            instance_name,
            driver,
            runtime_options,
            on_stage,
            self.addons,
            self.patch_mode,
        );
        let prepared = self.services.prepare_in_session(&session, params).await;
        finish_session_result(session, prepared).await
    }

    /// Installs persistent addons onto an existing local instance without re-running driver preparation.
    pub async fn install_addons(
        self,
        instance_name: impl Into<String>,
    ) -> Result<PreparedInstance> {
        let instance_name = instance_name.into();
        let session = open_or_reuse_session(
            self.downloader,
            self.session,
            &format!("install addons {instance_name}"),
        )
        .await?;
        let default_runtime = RuntimeOptions::default();
        let runtime_options = self.runtime_options.unwrap_or(&default_runtime);
        let prepared = self
            .services
            .install_addons_in_session(
                &session,
                instance_name,
                runtime_options,
                self.addons,
                self.patch_mode,
            )
            .await;
        finish_session_result(session, prepared).await
    }

    /// Resolves all active addons (persistent + launch-scoped) from the clean
    /// baseline and returns the unified [`PatchTrace`].
    pub async fn trace_launch(self, prepared: &PreparedInstance) -> Result<PatchTrace> {
        ensure_virtual_launch_policy(self.patch_mode)?;
        let default_runtime = RuntimeOptions::default();
        let runtime_options = self.runtime_options.unwrap_or(&default_runtime);
        let (mut entries, maybe_trace) = resolve_addons_for_launch(
            &self.services,
            self.downloader,
            prepared,
            runtime_options,
            self.session,
            self.addons,
        )
        .await?;
        match maybe_trace {
            Some(trace) => Ok(trace),
            None => order_and_trace_launch_entries(prepared, &mut entries),
        }
    }

    /// Builds the [`LaunchCommand`] with unified persistent and launch-scoped addons applied.
    pub async fn build_launch_command(
        self,
        prepared: &PreparedInstance,
        credential: &Credential,
        options: &LaunchOptions,
    ) -> Result<LaunchCommand> {
        ensure_virtual_launch_policy(self.patch_mode)?;
        let default_runtime = RuntimeOptions::default();
        let runtime_options = self.runtime_options.unwrap_or(&default_runtime);
        let mut launch_options = options.clone();

        let runtime = prepared
            .resolve_runtime(runtime_options, self.services.runtime_resolver())
            .await?;
        let (entries, trace) = resolve_addons_for_launch(
            &self.services,
            self.downloader,
            prepared,
            runtime_options,
            self.session,
            self.addons,
        )
        .await?;
        apply_launch_addon_hooks(prepared, &entries, credential, &mut launch_options).await?;
        prepared.build_launch_command(
            credential,
            runtime,
            &launch_options,
            trace.map(|t| t.metadata),
        )
    }

    /// Builds the launch command and spawns the game process.
    pub async fn launch_prepared(
        self,
        prepared: &PreparedInstance,
        credential: &Credential,
        options: &LaunchOptions,
    ) -> Result<LaunchedInstance> {
        let command = self
            .build_launch_command(prepared, credential, options)
            .await?;
        ops::spawn_launch_command(command)
    }
}

fn ensure_virtual_launch_policy(patch_mode: Option<PatchMode>) -> Result<()> {
    if patch_mode == Some(PatchMode::Physical) {
        return Err(elemental_addon::Error::PatchModeDisallowedByInstance {
            id: "<launch-scope>".to_owned(),
            instance_policy: PatchMode::Virtual,
        }
        .into());
    }
    Ok(())
}

async fn open_or_reuse_session(
    downloader: &ElementalDownloader,
    session: Option<DownloadSession>,
    label: &str,
) -> Result<DownloadSession> {
    match session {
        Some(session) => Ok(session),
        None => Ok(downloader.session(label).open().await?),
    }
}

async fn resolve_addons_for_launch(
    services: &LauncherServices<'_>,
    downloader: &ElementalDownloader,
    prepared: &PreparedInstance,
    runtime_options: &RuntimeOptions,
    session: Option<DownloadSession>,
    addons: Vec<RequestedAddon>,
) -> Result<(Vec<LaunchHookEntry>, Option<PatchTrace>)> {
    resolve_launch_addons(
        services.addons(),
        downloader,
        services.runtime_resolver(),
        prepared,
        runtime_options,
        session,
        addons,
    )
    .await
}

async fn finish_session_result(
    session: DownloadSession,
    prepared: Result<PreparedInstance>,
) -> Result<PreparedInstance> {
    let report = session.finish().await;
    let prepared = prepared?;
    let report = report?;
    report.success()?;
    Ok(prepared)
}
