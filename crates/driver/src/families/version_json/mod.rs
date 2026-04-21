pub mod builder;
pub mod classpath;
pub mod extensions;
pub mod layout;
pub mod meta;
pub mod platform;
pub mod prepared;
pub mod profile;
pub mod remote;
pub mod resource;
pub mod rules;
mod state;
pub mod storage;
pub mod variables;

pub use extensions::{PistonMetaDataExt, PistonMetaLibrariesExt};
pub use layout::{
    BaseInstanceLayout, BaseRootLayout, VersionJsonInstanceLayout, VersionJsonRootLayout,
};
pub use meta::{
    ContinuousArgument, LaunchMetaData, LaunchMetaLatestData, LaunchMetaVersionData,
    OperatingSystem, PistonMetaArguments, PistonMetaAssetIndex, PistonMetaAssetIndexObject,
    PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaDownload, PistonMetaDownloads,
    PistonMetaGenericArgument, PistonMetaJavaVersion, PistonMetaLibraries,
    PistonMetaLibrariesDownloads, PistonMetaLibrariesDownloadsArtifact, PistonMetaLibrariesExtract,
    PistonMetaLogging, PistonMetaLoggingSide, PistonMetaLoggingSideFile, PistonMetaRuleArgument,
    PistonMetaRuleArgumentRules,
};
pub use platform::VersionJsonPlatform;
pub use prepared::{
    LaunchedVersionJsonInstance, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
    ResolvedVersionJsonMetadata, VersionJsonInstallStatus,
};
pub use profile::{
    LibraryReplacementFamily, PASSTHROUGH_PROFILE_BEHAVIOR, PassthroughProfileBehavior,
    ProfileMergeBehavior, merge_profile_with_behavior, metadata_has_replaced_library_conflicts,
};
pub use remote::VersionJsonRemoteResolver;
pub use resource::{VersionJsonInstanceResource, VersionJsonRootResource};
pub use rules::{
    OperatingSystemExt, PistonMetaRuleExt, PistonMetaRulesExt, VersionJsonRuleContext,
};
pub use storage::{VersionJsonGameStorageExt, VersionJsonVersionStorageExt, inspect_instances};
