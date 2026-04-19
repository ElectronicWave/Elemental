use anyhow::Result;

use crate::downloader::core::{DownloadPlan, ElementalDownloader, SessionExecutionReport};

pub trait DownloadPlanner {
    fn plan(&self) -> Result<Vec<DownloadPlan>>;
}

pub async fn execute_plan(
    downloader: &ElementalDownloader,
    plan: DownloadPlan,
) -> Result<SessionExecutionReport> {
    downloader.run_plan(plan).await
}

pub async fn execute_plans(
    downloader: &ElementalDownloader,
    plans: Vec<DownloadPlan>,
) -> Result<Vec<SessionExecutionReport>> {
    let mut reports = Vec::with_capacity(plans.len());
    for plan in plans {
        reports.push(downloader.run_plan(plan).await?);
    }
    Ok(reports)
}

pub async fn execute_planner<P>(
    planner: &P,
    downloader: &ElementalDownloader,
) -> Result<Vec<SessionExecutionReport>>
where
    P: DownloadPlanner + ?Sized,
{
    let plans = planner.plan()?;
    execute_plans(downloader, plans).await
}
