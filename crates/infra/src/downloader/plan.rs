use anyhow::Result;

use crate::downloader::core::DownloadPlan;

pub trait DownloadPlanner {
    fn plan(&self) -> Result<Vec<DownloadPlan>>;
}
