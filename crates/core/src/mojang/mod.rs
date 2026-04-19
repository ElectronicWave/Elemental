mod base_url;
mod extensions;
mod platform;
mod rules;

pub use base_url::MojangBaseUrl;
pub use extensions::{PistonMetaDataExt, PistonMetaLibrariesExt};
pub use platform::MojangPlatform;
pub use rules::{MojangRuleContext, OperatingSystemExt, PistonMetaRuleExt, PistonMetaRulesExt};

pub use elemental_schema::mojang::launcher::{
    LaunchMetaData, LaunchMetaLatestData, LaunchMetaVersionData,
};
pub use elemental_schema::mojang::piston::{
    ContinuousArgument, OperatingSystem, PistonMetaArguments, PistonMetaAssetIndex,
    PistonMetaAssetIndexObject, PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaDownload,
    PistonMetaDownloads, PistonMetaGenericArgument, PistonMetaJavaVersion, PistonMetaLibraries,
    PistonMetaLibrariesDownloads, PistonMetaLibrariesDownloadsArtifact, PistonMetaLibrariesExtract,
    PistonMetaLogging, PistonMetaLoggingSide, PistonMetaLoggingSideFile, PistonMetaRuleArgument,
    PistonMetaRuleArgumentRules,
};
