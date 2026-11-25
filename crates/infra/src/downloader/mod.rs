mod config;
mod core;
mod event;
pub(crate) mod limits;
mod materializer;
mod report;
mod session;
mod state;
mod task;
mod transfer;
pub(crate) mod validation;

pub use config::{
    ByteRate, DownloadRateLimit, ElementalDownloaderConfig, FailurePolicy, RetryPolicy,
};
pub use core::{ElementalDownloader, ElementalDownloaderBuilder};
pub use event::{
    DownloadEvent, DownloadEventStream, EventStreamError, SessionEventStream, SessionSnapshot,
    TaskInfo, TaskSnapshot,
};
pub use materializer::{
    HardlinkCacheMaterializer, Materializer, StagedDownload, UncachedMaterializer,
};
pub use report::{
    ActiveTask, DownloadFailure, DownloadPhase, DownloadProgress, DownloadReport, TaskOutcome,
    TaskStatus,
};
pub use session::{DownloadSession, DownloadSessionBuilder};
pub use task::{DownloadBatch, DownloadTask, SessionId, TaskId};
