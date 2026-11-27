mod arguments;
pub mod builder;
pub mod classpath;
pub mod extensions;
pub mod layout;
pub mod patch;
pub mod platform;
pub mod prepared;
pub mod profile;
pub mod remote;
pub mod resource;
pub mod rules;
mod source;
pub mod storage;
pub mod variables;

pub use arguments::parse_argument_string;
pub use extensions::{PistonMetaDataExt, PistonMetaLibrariesExt};
pub use layout::{
    BaseInstanceLayout, BaseRootLayout, GameStorage, InstanceStorage, SharedInstanceLayout,
    SharedRootLayout, VersionJsonInstanceLayout, VersionJsonRootLayout,
};
pub use patch::{
    MetadataPatch, MetadataPatchStepDiff, NamedMetadataPatch, PatchMode, PatchTrace,
    apply_metadata_patches, apply_metadata_patches_traced, apply_patches_with_mode,
    apply_scoped_patches, read_base_metadata,
};
pub use platform::VersionJsonPlatform;
pub use prepared::{
    PreparedVersionJsonInstance, ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata,
    VersionJsonDownloadStatus, VersionJsonInstallStatus, resolve_vanilla_metadata,
};
pub use profile::{
    LibraryReplacementFamily, PASSTHROUGH_PROFILE_BEHAVIOR, PassthroughProfileBehavior,
    ProfileMergeBehavior, merge_piston_meta_arguments, merge_profile_with_behavior,
    metadata_has_replaced_library_conflicts, profile_library_to_piston,
};
pub use remote::{
    UpstreamUrlRewriter, VanillaEndpoints, VanillaFallbackRemoteResolver, VanillaOrigin,
    VanillaSource, VersionJsonRemoteResolver, rewrite_upstream_with_vanilla_fallback,
};
pub use resource::{VersionJsonInstanceResource, VersionJsonRootResource};
pub use rules::{
    OperatingSystemExt, PistonMetaRuleExt, PistonMetaRulesExt, VersionJsonRuleContext,
};
pub use source::{
    LoaderMetaCatalog, LoaderMetaEndpoints, LoaderMetaSource, LoaderProfileEndpoints,
};
pub use storage::{
    VersionJsonAssetIndexStorageExt, VersionJsonGameStorageExt, VersionJsonMetadataStorageExt,
};
