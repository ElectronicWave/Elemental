pub mod addon;
pub mod builtins;
pub mod error;
pub mod state;

pub use addon::{
    Addon, AddonCatalogQuery, AddonCompatibility, AddonContext, AddonDescriptor, AddonLaunchArgs,
    AddonLaunchContext, AddonOrigin, AddonPatchMode, AddonPrepareOutput, AddonPrepareRequest,
    AddonRelease, InstalledAddon, PatchModeSupport, resolve_effective_patch_mode,
};
#[cfg(feature = "authlib-injector")]
pub use builtins::{
    AUTHLIB_INJECTOR_ADDON_ID, AUTHLIB_INJECTOR_OFFICIAL_URL, AuthlibInjector,
    AuthlibInjectorAddon, AuthlibInjectorEndpoints, AuthlibInjectorSource,
};
pub use builtins::{CustomPatchAddon, JvmAgentAddon};
#[cfg(feature = "liteloader")]
pub use builtins::{
    LITELOADER_ADDON_ID, LiteLoaderAddon, LiteLoaderCatalog, LiteLoaderEndpoints, LiteLoaderOrigin,
    LiteLoaderRelease, LiteLoaderSource,
};
pub use elemental_driver::catalog::GameVersion;
pub use elemental_driver::families::version_json::{
    MetadataPatch, MetadataPatchStepDiff, NamedMetadataPatch, PatchMode, PatchTrace,
};
pub use elemental_schema::mojang::piston::{
    PistonMetaData, PistonMetaGenericArgument, PistonMetaLibraries, PistonMetaLogging,
};
pub use error::{Error, Result};
pub use state::{
    AddonState, InstalledAddonRecord, load_addon_state, normalize_managed_file_path,
    remove_addon_from_instance, reorder_instance_addons, save_addon_state, set_addon_patch_mode,
    set_instance_patch_mode, sync_instance_patches, trace_instance_patches,
};
