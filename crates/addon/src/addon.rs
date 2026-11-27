use std::{borrow::Cow, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use elemental_core::{
    auth::credential::Credential, minecraft::MinecraftVersionId, runtime::RuntimeResolver,
};
use elemental_driver::{
    catalog::GameVersion,
    driver::{DriverDescriptor, InstanceProbe},
    families::version_json::{InstanceStorage, MetadataPatch, PatchMode},
    launch::RuntimeOptions,
};
use elemental_infra::downloader::DownloadSession;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    state::InstalledAddonRecord,
};

/// Identifies an addon type by stable machine ID and human-readable display name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AddonDescriptor {
    pub id: Cow<'static, str>,
    pub name: Cow<'static, str>,
}

impl AddonDescriptor {
    pub fn new(id: impl Into<Cow<'static, str>>, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// Which patch modes an [`Addon`] is capable of supporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchModeSupport {
    VirtualOnly,
    PhysicalOnly,
    #[default]
    Both,
}

/// Per-addon patch mode preference, monotonically constrained by the instance-level [`PatchMode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddonPatchMode {
    #[default]
    Inherit,
    Virtual,
    Physical,
}

/// Resolves the effective [`PatchMode`] for an addon given the instance-level policy,
/// the addon's per-addon preference, and the addon's declared capability.
///
/// Enforces monotonic tightening: an addon can tighten a `Physical` instance policy down
/// to `Virtual`, but can never elevate a `Virtual` instance policy to `Physical`.
pub fn resolve_effective_patch_mode(
    addon_id: &str,
    instance_policy: PatchMode,
    addon_mode: AddonPatchMode,
    support: PatchModeSupport,
) -> Result<PatchMode> {
    if instance_policy == PatchMode::Virtual
        && (addon_mode == AddonPatchMode::Physical || support == PatchModeSupport::PhysicalOnly)
    {
        return Err(Error::PatchModeDisallowedByInstance {
            id: addon_id.to_owned(),
            instance_policy,
        });
    }

    match (addon_mode, support) {
        (AddonPatchMode::Physical, PatchModeSupport::VirtualOnly) => {
            Err(Error::PatchModeUnsupported {
                id: addon_id.to_owned(),
                requested: PatchMode::Physical,
                supported: "VirtualOnly".to_owned(),
            })
        }
        (AddonPatchMode::Virtual, PatchModeSupport::PhysicalOnly) => {
            Err(Error::PatchModeUnsupported {
                id: addon_id.to_owned(),
                requested: PatchMode::Virtual,
                supported: "PhysicalOnly".to_owned(),
            })
        }
        (AddonPatchMode::Virtual, _) => Ok(PatchMode::Virtual),
        (AddonPatchMode::Physical, _) => Ok(PatchMode::Physical),
        (AddonPatchMode::Inherit, PatchModeSupport::VirtualOnly) => Ok(PatchMode::Virtual),
        (AddonPatchMode::Inherit, PatchModeSupport::PhysicalOnly) => Ok(PatchMode::Physical),
        (AddonPatchMode::Inherit, PatchModeSupport::Both) => Ok(instance_policy),
    }
}

/// Whether an installed addon is managed by Elemental or detected from an external launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddonOrigin {
    Elemental,
    External,
}

/// Result of checking whether an addon can be applied to a host driver and game version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddonCompatibility {
    Compatible,
    Incompatible { reason: String },
}

/// Read-only context passed to [`Addon::check_compatibility`].
pub struct AddonContext<'a> {
    pub driver: DriverDescriptor,
    pub game_version: Option<&'a MinecraftVersionId>,
    pub instance: &'a InstanceStorage,
}

/// Inputs passed to [`Addon::prepare`].
pub struct AddonPrepareRequest<'a> {
    pub session: &'a DownloadSession,
    pub instance: &'a InstanceStorage,
    pub driver: DriverDescriptor,
    pub game_version: &'a MinecraftVersionId,
    pub addon_version: Option<&'a str>,
    pub runtime_options: &'a RuntimeOptions,
    pub runtime_resolver: &'a RuntimeResolver,
}

/// Output produced by [`Addon::prepare`].
#[derive(Debug, Clone, Default)]
pub struct AddonPrepareOutput {
    pub version: Option<String>,
    pub metadata_patch: Option<MetadataPatch>,
    pub managed_files: Vec<PathBuf>,
}

/// Launch-time argument injections produced by [`Addon::augment_launch`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddonLaunchArgs {
    pub extra_jvm_arguments: Vec<String>,
    pub extra_game_arguments: Vec<String>,
}

/// Context passed to [`Addon::augment_launch`].
pub struct AddonLaunchContext<'a> {
    pub instance: &'a InstanceStorage,
    pub driver: DriverDescriptor,
    pub credential: &'a Credential,
    pub record: &'a InstalledAddonRecord,
}

/// Summary of an addon installed or detected on an instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledAddon {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub origin: AddonOrigin,
    #[serde(default)]
    pub effective_patch_mode: PatchMode,
    #[serde(default)]
    pub requires_runtime_hook: bool,
}

/// A catalog entry describing an available version of an [`Addon`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AddonRelease {
    pub game_version: MinecraftVersionId,
    pub version: String,
    pub channel: Option<String>,
    pub published_at: Option<String>,
    pub prerelease: Option<bool>,
}

impl AddonRelease {
    pub fn new(game_version: MinecraftVersionId, version: impl Into<String>) -> Self {
        Self {
            game_version,
            version: version.into(),
            channel: None,
            published_at: None,
            prerelease: None,
        }
    }

    pub fn with_channel(mut self, channel: Option<String>) -> Self {
        self.channel = channel;
        self
    }

    pub fn with_published_at(mut self, published_at: Option<String>) -> Self {
        self.published_at = published_at;
        self
    }

    pub fn with_prerelease(mut self, prerelease: Option<bool>) -> Self {
        self.prerelease = prerelease;
        self
    }
}

/// Host driver and game version context used when querying an addon's version catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddonCatalogQuery<'a> {
    pub driver: DriverDescriptor,
    pub game_version: &'a MinecraftVersionId,
}

/// Composable, host-aware extension layered onto a Minecraft instance.
#[async_trait]
pub trait Addon: Send + Sync {
    fn descriptor(&self) -> AddonDescriptor;

    /// Declares which patch modes this addon supports.
    fn patch_mode_support(&self) -> PatchModeSupport {
        PatchModeSupport::Both
    }

    /// Returns `true` when this addon requires code execution during launch
    /// command construction (`augment_launch`) and cannot degrade to data-only.
    fn requires_runtime_hook(&self) -> bool {
        false
    }

    /// Returns supported game versions when this addon exposes a remote catalog.
    async fn supported_game_versions(&self) -> Result<Option<Vec<GameVersion>>> {
        Ok(None)
    }

    /// Returns available addon releases for a host driver and game version, or
    /// `None` when this addon does not provide a version catalog.
    async fn available_versions(
        &self,
        _query: &AddonCatalogQuery<'_>,
    ) -> Result<Option<Vec<AddonRelease>>> {
        Ok(None)
    }

    fn check_compatibility(&self, _ctx: &AddonContext<'_>) -> AddonCompatibility {
        AddonCompatibility::Compatible
    }

    async fn inspect(&self, _probe: &InstanceProbe) -> Result<Option<InstalledAddon>> {
        Ok(None)
    }

    async fn prepare(&self, _request: &AddonPrepareRequest<'_>) -> Result<AddonPrepareOutput> {
        Ok(AddonPrepareOutput::default())
    }

    async fn augment_launch(
        &self,
        _args: &mut AddonLaunchArgs,
        _ctx: &AddonLaunchContext<'_>,
    ) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl<T: Addon + ?Sized> Addon for Arc<T> {
    fn descriptor(&self) -> AddonDescriptor {
        (**self).descriptor()
    }

    fn patch_mode_support(&self) -> PatchModeSupport {
        (**self).patch_mode_support()
    }

    fn requires_runtime_hook(&self) -> bool {
        (**self).requires_runtime_hook()
    }

    async fn supported_game_versions(&self) -> Result<Option<Vec<GameVersion>>> {
        (**self).supported_game_versions().await
    }

    async fn available_versions(
        &self,
        query: &AddonCatalogQuery<'_>,
    ) -> Result<Option<Vec<AddonRelease>>> {
        (**self).available_versions(query).await
    }

    fn check_compatibility(&self, ctx: &AddonContext<'_>) -> AddonCompatibility {
        (**self).check_compatibility(ctx)
    }

    async fn inspect(&self, probe: &InstanceProbe) -> Result<Option<InstalledAddon>> {
        (**self).inspect(probe).await
    }

    async fn prepare(&self, request: &AddonPrepareRequest<'_>) -> Result<AddonPrepareOutput> {
        (**self).prepare(request).await
    }

    async fn augment_launch(
        &self,
        args: &mut AddonLaunchArgs,
        ctx: &AddonLaunchContext<'_>,
    ) -> Result<()> {
        (**self).augment_launch(args, ctx).await
    }
}
