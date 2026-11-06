use std::time::Duration;

#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DownloadProgress {
    pub tasks_total: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub tasks_skipped: usize,
    pub tasks_cancelled: usize,
    pub tasks_pending: usize,
    /// Bytes successfully written to their final destinations.
    pub bytes_downloaded: u64,
    /// Bytes received from the network, including bytes from retried attempts.
    pub bytes_received: u64,
    /// Bytes that did not need to be downloaded.
    pub bytes_skipped: u64,
    pub expected_bytes: Option<u64>,
    pub bytes_per_second: u64,
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "serialize_optional_duration_millis")
    )]
    pub eta: Option<Duration>,
}

#[cfg(feature = "serde")]
fn serialize_optional_duration_millis<S>(
    duration: &Option<Duration>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match duration {
        Some(duration) => super::serialize_duration_millis(duration, serializer),
        None => serializer.serialize_none(),
    }
}
