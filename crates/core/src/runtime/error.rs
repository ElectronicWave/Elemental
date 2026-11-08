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

    #[error(
        "can't determine Java major version for explicit runtime distribution: {}",
        path.display()
    )]
    RuntimeVersionUnknown { path: PathBuf },

    #[error("can't find a local Java runtime with major version {major_version}")]
    RuntimeNotFound { major_version: usize },

    #[error(
        "explicit runtime distribution has Java major version {actual}, expected {expected}: {}",
        path.display()
    )]
    VersionMismatch {
        actual: usize,
        expected: usize,
        path: PathBuf,
    },

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
