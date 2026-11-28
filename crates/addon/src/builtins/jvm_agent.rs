use std::path::PathBuf;

use async_trait::async_trait;

use crate::{
    addon::{
        Addon, AddonDescriptor, AddonLaunchArgs, AddonLaunchContext, AddonPrepareOutput,
        AddonPrepareRequest, PatchModeSupport,
    },
    error::Result,
};

/// Addon that injects a `-javaagent` and optional JVM arguments at launch time.
#[derive(Debug, Clone)]
pub struct JvmAgentAddon {
    descriptor: AddonDescriptor,
    version: Option<String>,
    jar_path: PathBuf,
    agent_option: Option<String>,
    extra_jvm_arguments: Vec<String>,
}

impl JvmAgentAddon {
    /// Creates a JVM agent addon injecting `-javaagent:<jar_path>` at launch time.
    pub fn new(
        id: impl Into<std::borrow::Cow<'static, str>>,
        name: impl Into<std::borrow::Cow<'static, str>>,
        jar_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            descriptor: AddonDescriptor::new(id, name),
            version: None,
            jar_path: jar_path.into(),
            agent_option: None,
            extra_jvm_arguments: Vec::new(),
        }
    }

    /// Sets the default version string recorded when the caller omits one.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Attaches an agent option string formatted as `-javaagent:<jar_path>=<option>`.
    pub fn with_agent_option(mut self, option: impl Into<String>) -> Self {
        self.agent_option = Some(option.into());
        self
    }

    /// Appends additional JVM arguments injected alongside `-javaagent`.
    pub fn with_extra_jvm_arguments(mut self, args: Vec<String>) -> Self {
        self.extra_jvm_arguments = args;
        self
    }

    fn launch_jvm_arguments(&self) -> Vec<String> {
        let javaagent_arg = match &self.agent_option {
            Some(option) => format!("-javaagent:{}={option}", self.jar_path.display()),
            None => format!("-javaagent:{}", self.jar_path.display()),
        };
        let mut args = Vec::with_capacity(1 + self.extra_jvm_arguments.len());
        args.push(javaagent_arg);
        args.extend(self.extra_jvm_arguments.iter().cloned());
        args
    }
}

#[async_trait]
impl Addon for JvmAgentAddon {
    fn descriptor(&self) -> AddonDescriptor {
        self.descriptor.clone()
    }

    fn patch_mode_support(&self) -> PatchModeSupport {
        PatchModeSupport::VirtualOnly
    }

    fn requires_runtime_hook(&self) -> bool {
        true
    }

    async fn prepare(&self, request: &AddonPrepareRequest<'_>) -> Result<AddonPrepareOutput> {
        Ok(AddonPrepareOutput {
            version: request
                .addon_version
                .map(str::to_owned)
                .or_else(|| self.version.clone()),
            metadata_patch: None,
            managed_files: Vec::new(),
        })
    }

    async fn augment_launch(
        &self,
        args: &mut AddonLaunchArgs,
        _ctx: &AddonLaunchContext<'_>,
    ) -> Result<()> {
        args.extra_jvm_arguments.extend(self.launch_jvm_arguments());
        Ok(())
    }
}
