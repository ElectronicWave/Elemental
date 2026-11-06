mod batch;
mod config;
mod core;
mod event;
mod header;
pub(crate) mod limits;
mod materializer;
mod policy;
mod rate;
mod report;
mod session;
mod state;
mod task;
mod transfer;
pub(crate) mod validation;
mod worker;

pub use batch::DownloadBatch;
pub use config::ElementalDownloaderConfig;
pub use core::{ElementalDownloader, ElementalDownloaderBuilder};
pub use event::{
    DownloadEvent, DownloadEventStream, EventStreamError, SessionEventStream, SessionSnapshot,
    TaskInfo, TaskSnapshot,
};
pub use materializer::{
    HardlinkCacheMaterializer, Materializer, StagedDownload, UncachedMaterializer,
};
pub use policy::{FailurePolicy, RetryPolicy};
pub use rate::{ByteRate, DownloadRateLimit};
pub use report::{
    DownloadFailure, DownloadPhase, DownloadProgress, DownloadReport, TaskOutcome, TaskStatus,
};
pub use session::{DownloadSession, DownloadSessionBuilder};
pub use task::{DownloadTask, SessionId, TaskId};
