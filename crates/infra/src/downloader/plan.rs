use crate::downloader::core::DownloadPlan;
use crate::error::Result;

pub trait DownloadPlanner {
    fn plan(&self) -> Result<Vec<DownloadPlan>>;
}
