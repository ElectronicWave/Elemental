use std::{io, path::PathBuf, process::ExitStatus};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(
        "Java property probe timed out after {timeout_secs} seconds: {}",
        executable.display()
    )]
    ProbeTimeout {
        executable: PathBuf,
        timeout_secs: u64,
    },

    #[error("run Java property probe failed: {}", executable.display())]
    ProbeSpawn {
        executable: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "Java property probe exited unsuccessfully with {status}: {}",
        executable.display()
    )]
    ProbeFailed {
        executable: PathBuf,
        status: ExitStatus,
    },

    #[error(
        "Java property probe returned no recognizable major version: {}",
        executable.display()
    )]
    ProbeMissingVersion { executable: PathBuf },

    #[error("read Java release file failed: {}", path.display())]
    ReadRelease {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("check Java release file failed: {}", path.display())]
    CheckRelease {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Java release file has no recognizable major version: {}", path.display())]
    ReleaseMissingVersion { path: PathBuf },

    #[error("canonicalize Java runtime root failed: {}", path.display())]
    CanonicalizeRuntimeRoot {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("canonicalize runtime executable failed: {}", path.display())]
    CanonicalizeExecutable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("resolve current directory for runtime executable failed")]
    CurrentDir {
        #[source]
        source: io::Error,
    },

    #[error("runtime executable path is not a file: {}", path.display())]
    ExecutableNotFile { path: PathBuf },

    #[error("runtime executable path has no valid file stem: {}", path.display())]
    ExecutableNoStem { path: PathBuf },

    #[error("runtime executable must point to java or javaw: {}", path.display())]
    ExecutableNotJava { path: PathBuf },

    #[error("runtime executable has no parent directory: {}", path.display())]
    ExecutableNoParent { path: PathBuf },

    #[error("runtime executable is not inside a bin directory: {}", path.display())]
    ExecutableNotInBin { path: PathBuf },

    #[error("determine Java runtime properties failed: {}", path.display())]
    DetermineRuntimeProperties {
        path: PathBuf,
        #[source]
        source: Box<Error>,
    },

    #[error(
        "build Java runtime distribution from {provider} at '{}' failed",
        path.display()
    )]
    BuildDistribution {
        provider: &'static str,
        path: PathBuf,
        #[source]
        source: Box<Error>,
    },

    #[error("resolve {usage} runtime from executable failed: {}", path.display())]
    ResolveRuntime {
        usage: String,
        path: PathBuf,
        #[source]
        source: Box<Error>,
    },

    #[error("can't determine Java major version for explicit {usage} runtime executable: {}", path.display())]
    RuntimeVersionUnknown { usage: String, path: PathBuf },

    #[error("can't find a local Java runtime with major version {major_version} for {usage}")]
    RuntimeNotFound { major_version: usize, usage: String },

    #[error(
        "explicit {usage} runtime executable has Java major version {actual}, expected {expected}: {}",
        path.display()
    )]
    VersionMismatch {
        usage: String,
        actual: usize,
        expected: usize,
        path: PathBuf,
    },

    #[error("lock runtime provider override for writing failed")]
    ProviderLockPoisoned,

    #[error("custom runtime path is not a directory: {}", path.display())]
    CustomRuntimeNotDirectory { path: PathBuf },

    #[error("custom runtime path does not contain a java executable: {}", path.display())]
    CustomRuntimeMissingJava { path: PathBuf },

    #[error("canonicalize custom runtime path failed: {}", path.display())]
    CanonicalizeCustomRuntime {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
