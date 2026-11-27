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
    /// Stable machine-readable identifier (for example, `"liteloader"`).
    pub id: Cow<'static, str>,
    /// Human-readable display name.
    pub name: Cow<'static, str>,
}

impl AddonDescriptor {
    /// Creates a new addon descriptor from a machine ID and display name.
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
    /// Only supports launch-time virtual overlays (`vpatch`) and/or runtime hooks.
    VirtualOnly,
    /// Requires physical writes into the instance's `version.json`.
    PhysicalOnly,
    /// Supports both virtual overlays (`vpatch`) and physical `version.json` patching.
    #[default]
    Both,
}

/// Per-addon patch mode preference, monotonically constrained by the instance-level [`PatchMode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddonPatchMode {
    /// Follow the instance's [`PatchMode`] policy (default).
    #[default]
    Inherit,
    /// Force virtual patching (`vpatch`) regardless of a `Physical` instance policy.
    Virtual,
    /// Request physical patching into `version.json` (rejected if instance policy is `Virtual`).
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
    /// Installed and tracked by Elemental in `.elemental/addons.json`.
    Elemental,
    /// Detected from an existing instance's metadata or files without an Elemental state record.
    External,
}

/// Result of checking whether an addon can be applied to a host driver and game version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddonCompatibility {
    /// The addon can be applied to the target host instance.
    Compatible,
    /// The addon cannot be applied to the target host instance.
    Incompatible {
        /// Human-readable explanation of why the addon is incompatible.
        reason: String,
    },
}

/// Read-only context passed to [`Addon::check_compatibility`].
pub struct AddonContext<'a> {
    /// Host driver installed on the target instance.
    pub driver: DriverDescriptor,
    /// Target Minecraft version, when known.
    pub game_version: Option<&'a MinecraftVersionId>,
    /// Storage handle for the target instance directory.
    pub instance: &'a InstanceStorage,
}

/// Inputs passed to [`Addon::prepare`].
pub struct AddonPrepareRequest<'a> {
    /// Active download session used to download addon artifacts.
    pub session: &'a DownloadSession,
    /// Storage handle for the target instance directory.
    pub instance: &'a InstanceStorage,
    /// Host driver installed on the target instance.
    pub driver: DriverDescriptor,
    /// Minecraft version of the target instance.
    pub game_version: &'a MinecraftVersionId,
    /// Requested addon version, if specified by the caller.
    pub addon_version: Option<&'a str>,
    /// Runtime options configured for installation/launch preparation.
    pub runtime_options: &'a RuntimeOptions,
    /// Runtime resolver for locating Java installations when needed.
    pub runtime_resolver: &'a RuntimeResolver,
}

/// Output produced by [`Addon::prepare`].
#[derive(Debug, Clone, Default)]
pub struct AddonPrepareOutput {
    /// Resolved addon version string to persist in instance state.
    pub version: Option<String>,
    /// Declarative metadata patch layered onto the instance's `PistonMetaData`.
    pub metadata_patch: Option<MetadataPatch>,
    /// Additional files created/managed by this addon (tracked for cleanup on removal).
    pub managed_files: Vec<PathBuf>,
}

/// Launch-time argument injections produced by [`Addon::augment_launch`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddonLaunchArgs {
    /// Additional JVM flags appended before the main class.
    pub extra_jvm_arguments: Vec<String>,
    /// Additional game arguments appended after the host driver's game arguments.
    pub extra_game_arguments: Vec<String>,
}

/// Context passed to [`Addon::augment_launch`].
pub struct AddonLaunchContext<'a> {
    /// Storage handle for the instance being launched.
    pub instance: &'a InstanceStorage,
    /// Host driver of the instance being launched.
    pub driver: DriverDescriptor,
    /// Authenticated player credential for this launch.
    pub credential: &'a Credential,
    /// Persisted state record for this addon on the instance.
    pub record: &'a InstalledAddonRecord,
}

/// Summary of an addon installed or detected on an instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledAddon {
    /// Machine-readable addon identifier.
    pub id: String,
    /// Human-readable addon display name.
    pub name: String,
    /// Installed addon version, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Whether this addon is Elemental-managed or externally detected.
    pub origin: AddonOrigin,
    /// Effective patch mode resolved from instance policy and per-addon preference.
    #[serde(default)]
    pub effective_patch_mode: PatchMode,
    /// Whether launching requires executing [`Addon::augment_launch`] in code.
    #[serde(default)]
    pub requires_runtime_hook: bool,
}

/// A catalog entry describing an available version of an [`Addon`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AddonRelease {
    /// Addon release version string.
    pub version: String,
    /// Optional release channel (for example, `"release"` or `"snapshot"`).
    pub channel: Option<String>,
    /// Optional publication timestamp string from the upstream source.
    pub published_at: Option<String>,
    /// Whether the upstream catalog marks this release as a prerelease.
    pub prerelease: Option<bool>,
}

impl AddonRelease {
    /// Creates a release entry for the given addon `version`.
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            channel: None,
            published_at: None,
            prerelease: None,
        }
    }

    /// Sets the release channel label.
    pub fn with_channel(mut self, channel: Option<String>) -> Self {
        self.channel = channel;
        self
    }

    /// Sets the publication timestamp string.
    pub fn with_published_at(mut self, published_at: Option<String>) -> Self {
        self.published_at = published_at;
        self
    }

    /// Sets whether this release is a prerelease.
    pub fn with_prerelease(mut self, prerelease: Option<bool>) -> Self {
        self.prerelease = prerelease;
        self
    }
}

/// Optional host driver and game version filters for querying an addon's version catalog.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AddonCatalogQuery<'a> {
    /// Optional host driver filter to query compatible addon releases for.
    pub driver: Option<DriverDescriptor>,
    /// Optional Minecraft version filter to query compatible addon releases for.
    pub game_version: Option<&'a MinecraftVersionId>,
}

impl<'a> AddonCatalogQuery<'a> {
    /// Creates an unfiltered catalog query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filters releases by host `driver`.
    pub fn with_driver(mut self, driver: DriverDescriptor) -> Self {
        self.driver = Some(driver);
        self
    }

    /// Filters releases by Minecraft `game_version`.
    pub fn with_game_version(mut self, game_version: &'a MinecraftVersionId) -> Self {
        self.game_version = Some(game_version);
        self
    }
}

/// Composable, host-aware extension layered onto a Minecraft instance.
#[async_trait]
pub trait Addon: Send + Sync {
    /// Returns the addon's unique identifier and display name.
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

    /// Checks whether this addon is compatible with the host driver and game version.
    fn check_compatibility(&self, _ctx: &AddonContext<'_>) -> AddonCompatibility {
        AddonCompatibility::Compatible
    }

    /// Detects whether an unmanaged/external copy of this addon is already present in `probe`.
    async fn inspect(&self, _probe: &InstanceProbe) -> Result<Option<InstalledAddon>> {
        Ok(None)
    }

    /// Downloads any required artifacts and produces the metadata patch and managed files
    /// to persist for this addon.
    async fn prepare(&self, _request: &AddonPrepareRequest<'_>) -> Result<AddonPrepareOutput> {
        Ok(AddonPrepareOutput::default())
    }

    /// Augments the launch command with dynamic JVM or game arguments before spawning.
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
