use std::{error::Error as StdError, io, path::PathBuf, time::SystemTimeError};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Xml(#[from] quick_xml::DeError),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Regex(#[from] regex::Error),

    #[error(transparent)]
    Infra(#[from] elemental_infra::error::Error),

    #[error(transparent)]
    Shared(#[from] elemental_shared::error::Error),

    #[error(transparent)]
    Storage(#[from] elemental_core::storage::Error),

    #[error(transparent)]
    Runtime(#[from] elemental_core::runtime::Error),

    #[error(transparent)]
    Launcher(#[from] elemental_core::launcher::Error),

    #[error(transparent)]
    SystemTime(#[from] SystemTimeError),

    #[error("{context}: {url}")]
    InvalidUrl {
        context: String,
        url: String,
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },

    #[error("{context}: {base_url} + {path}")]
    JoinUrl {
        context: String,
        base_url: String,
        path: String,
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },

    #[error("URL cannot be used as a path base: {url}")]
    UrlNotPathBase { url: String },

    #[error("can't map URL to a known origin: {raw_url}")]
    UnknownOrigin { raw_url: String },

    #[error("URL matches a known origin host but not a configured base path: {raw_url}")]
    OriginBasePathMismatch { raw_url: String },

    #[error("request {source_name} resource failed: {url}")]
    RequestResource {
        source_name: String,
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("{source_name} returned error status: {url}")]
    HttpStatus {
        source_name: String,
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("decode {source_name} resource failed: {url}")]
    DecodeResource {
        source_name: String,
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("invalid {label} notation: {notation}")]
    InvalidNotation { label: String, notation: String },

    #[error("decode {source_label} maven metadata failed: {url}")]
    DecodeMavenMetadata {
        source_label: String,
        url: String,
        #[source]
        source: quick_xml::DeError,
    },

    #[error("read version metadata failed")]
    ReadVersionMetadata {
        #[source]
        source: Box<Error>,
    },

    #[error("authorize failed")]
    Authorize {
        #[source]
        source: elemental_core::auth::Error,
    },

    #[error("get version name failed for '{}'", path.display())]
    MissingVersionName { path: PathBuf },

    #[error("version root has no file name: {}", path.display())]
    MissingVersionRootFileName { path: PathBuf },

    #[error("logging config not found: {}", path.display())]
    LoggingConfigMissing { path: PathBuf },

    #[error("module path argument is missing its value")]
    ModulePathValueMissing,

    #[error("resolve module path argument failed")]
    ResolveModulePath,

    #[error("version jar not found: {}", path.display())]
    VersionJarMissing { path: PathBuf },

    #[error("local version '{version}' is not prepared: {status}")]
    VersionNotPrepared { version: String, status: String },

    #[error("parse base minecraftArguments failed")]
    ParseBaseMinecraftArguments {
        #[source]
        source: Box<Error>,
    },

    #[error("launcher variables serialized to a non-object JSON value")]
    LauncherVariablesNotObject,

    #[error("can't find a valid instance named '{name}'")]
    InstanceNotFound { name: String },

    #[error("{context} has no parent directory: {}", path.display())]
    MissingParentDirectory { context: String, path: PathBuf },

    #[error(
        "copy native binary '{}' to '{}' failed",
        source_path.display(),
        target_path.display()
    )]
    CopyNativeBinary {
        source_path: PathBuf,
        target_path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("installer coordinate has no file name: {notation}")]
    InstallerCoordinateFileNameMissing { notation: String },

    #[error("read persisted {family_name} install profile failed: {}", path.display())]
    ReadPersistedInstallProfile {
        family_name: String,
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("read persisted {family_name} embedded version failed: {}", path.display())]
    ReadPersistedEmbeddedVersion {
        family_name: String,
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("{family_name} install profile is missing minecraft version")]
    InstallerProfileMinecraftVersionMissing { family_name: String },

    #[error("{family_name} install profile is missing {loader_name} version identity")]
    InstallerProfileLoaderVersionMissing {
        family_name: String,
        loader_name: String,
    },

    #[error(
        "{family_name} installer game version '{actual}' does not match requested game version '{expected}'"
    )]
    InstallerGameVersionMismatch {
        family_name: String,
        actual: String,
        expected: String,
    },

    #[error(
        "{family_name} installer loader version '{actual}' does not match requested loader version '{expected}'"
    )]
    InstallerLoaderVersionMismatch {
        family_name: String,
        actual: String,
        expected: String,
    },

    #[error("read {family_name} embedded version json failed: {path}")]
    ReadEmbeddedVersionJson {
        family_name: String,
        path: String,
        #[source]
        source: Box<Error>,
    },

    #[error("download {family_name} installer failed")]
    DownloadInstaller {
        family_name: String,
        #[source]
        source: elemental_infra::error::Error,
    },

    #[error("read {family_name} install_profile.json from installer failed")]
    ReadInstallerProfile {
        family_name: String,
        #[source]
        source: Box<Error>,
    },

    #[error("extract {family_name} embedded maven artifacts failed")]
    ExtractInstallerMavenArtifacts {
        family_name: String,
        #[source]
        source: Box<Error>,
    },

    #[error(
        "{family_name} installer is missing an embedded version json; launchable {family_name} preparation is not available"
    )]
    InstallerEmbeddedVersionMissing { family_name: String },

    #[error("decode {family_name} embedded version json failed")]
    DecodeEmbeddedVersion {
        family_name: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("local {family_name} instance '{instance}' is not fully prepared: {status}")]
    InstallerNotPrepared {
        family_name: String,
        instance: String,
        status: String,
    },

    #[error("get {family_name} instance name failed for '{}'", path.display())]
    MissingInstanceName { family_name: String, path: PathBuf },

    #[error("download {family_name} install profile libraries failed")]
    DownloadInstallerProfileLibraries {
        family_name: String,
        #[source]
        source: elemental_infra::error::Error,
    },

    #[error(
        "{family_name} client processors completed without producing the expected client artifacts"
    )]
    ProcessorOutputsMissing { family_name: String },

    #[error("unknown {family_name} processor variable: {key}")]
    UnknownProcessorVariable { family_name: String, key: String },

    #[error("run {family_name} processor failed: {processor}")]
    RunProcessor {
        family_name: String,
        processor: String,
        #[source]
        source: io::Error,
    },

    // ProcessorFailure is large; boxing it keeps every Result<T, Error> small.
    #[error(transparent)]
    ProcessorFailed(Box<ProcessorFailure>),

    #[error("jar manifest is missing Main-Class: {}", path.display())]
    JarMainClassMissing { path: PathBuf },

    #[error("resolve current directory failed for {}", path.display())]
    CurrentDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("asset hash is too short: '{hash}'")]
    AssetHashTooShort { hash: String },

    #[error("can't find vanilla version named '{game_version}'")]
    VanillaVersionNotFound { game_version: String },

    #[error("rewrite {family_name} upstream url failed for '{raw_url}'")]
    RewriteUpstream {
        family_name: String,
        raw_url: String,
        #[source]
        source: Box<Error>,
    },

    #[error("can't find LiteLoader Minecraft version '{game_version}'")]
    LiteLoaderMinecraftVersionNotFound { game_version: String },

    #[error(
        "can't find LiteLoader loader version '{loader_version}' for Minecraft '{game_version}'"
    )]
    LiteLoaderLoaderVersionNotFound {
        loader_version: String,
        game_version: String,
    },

    #[error("LiteLoader library '{library}' does not declare a supported repository")]
    LiteLoaderLibraryRepositoryMissing { library: String },

    #[error("LiteLoader snapshot library '{library}' does not declare a repository")]
    LiteLoaderSnapshotRepositoryMissing { library: String },

    #[error("can't find Rift release asset named '{asset_name}'")]
    RiftReleaseAssetNotFound { asset_name: String },

    #[error("Rift release tag has no loader version: '{tag_name}'")]
    RiftReleaseTagMissingLoaderVersion { tag_name: String },

    #[error("read embedded Rift profile failed from '{asset_name}'")]
    ReadRiftProfile {
        asset_name: String,
        #[source]
        source: elemental_infra::error::Error,
    },

    #[error("decode embedded Rift profile failed")]
    DecodeRiftProfile {
        #[source]
        source: serde_json::Error,
    },

    #[error(
        "Rift loader '{loader_version}' targets Minecraft '{actual}' but '{requested}' was requested"
    )]
    RiftGameVersionMismatch {
        loader_version: String,
        actual: String,
        requested: String,
    },

    #[error("trailing escape in argument string")]
    TrailingArgumentEscape,

    #[error("unterminated quoted argument string with quote {quote}")]
    UnterminatedArgumentQuote { quote: char },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error(
    "{family_name} processor failed: processor={processor}, java={java}, main_class={main_class}, classpath={classpath}, args={args:?}, exit_status={exit_status:?}, stdout=\n{stdout}\nstderr=\n{stderr}"
)]
pub struct ProcessorFailure {
    pub family_name: String,
    pub processor: String,
    pub java: String,
    pub main_class: String,
    pub classpath: String,
    pub args: Vec<String>,
    pub exit_status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Error {
    pub(crate) fn invalid_url(
        context: impl Into<String>,
        url: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::InvalidUrl {
            context: context.into(),
            url: url.into(),
            source: Box::new(source),
        }
    }

    pub(crate) fn join_url(
        context: impl Into<String>,
        base_url: impl Into<String>,
        path: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::JoinUrl {
            context: context.into(),
            base_url: base_url.into(),
            path: path.into(),
            source: Box::new(source),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
