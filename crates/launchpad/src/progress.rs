use std::{future::Future, time::Duration};

use anyhow::{Context, Result};
use elemental::infra::downloader::{DownloadProgress, DownloadSession};
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};

const SPINNER_TEMPLATE: &str = "{spinner:.green} {msg}";
const DOWNLOAD_TEMPLATE: &str =
    "{spinner:.green} {msg} [{bar:40.cyan/blue}] {human_pos}/{human_len} ({bytes_per_sec}, {eta})";

pub async fn with_spinner<T, E, F>(message: &str, operation: F) -> Result<T>
where
    E: Into<anyhow::Error>,
    F: Future<Output = std::result::Result<T, E>>,
{
    let progress_bar = spinner(message)?;
    let result = operation.await.map_err(Into::into);
    progress_bar.finish_and_clear();
    result
}

pub fn download(session: &DownloadSession) -> Result<DownloadProgressBar> {
    let style = ProgressStyle::with_template(DOWNLOAD_TEMPLATE)
        .context("invalid download progress template")?;
    let progress_bar = ProgressBar::new(0)
        .with_style(style)
        .with_message("Preparing instance");
    progress_bar.enable_steady_tick(Duration::from_millis(100));
    apply_download_progress(&progress_bar, &session.progress());

    let mut receiver = session.progress_receiver();
    let task_progress_bar = progress_bar.clone();
    let task = tokio::spawn(async move {
        while receiver.changed().await.is_ok() {
            let progress = receiver.borrow_and_update().clone();
            apply_download_progress(&task_progress_bar, &progress);
        }
    });

    Ok(DownloadProgressBar { progress_bar, task })
}

pub struct DownloadProgressBar {
    progress_bar: ProgressBar,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for DownloadProgressBar {
    fn drop(&mut self) {
        self.task.abort();
        self.progress_bar.finish_and_clear();
    }
}

fn spinner(message: &str) -> Result<ProgressBar> {
    let style =
        ProgressStyle::with_template(SPINNER_TEMPLATE).context("invalid spinner template")?;
    let progress_bar = ProgressBar::new_spinner()
        .with_style(style)
        .with_message(message.to_owned());
    progress_bar.enable_steady_tick(Duration::from_millis(100));
    Ok(progress_bar)
}

fn apply_download_progress(progress_bar: &ProgressBar, progress: &DownloadProgress) {
    let finished_tasks =
        progress.tasks_completed + progress.tasks_failed + progress.tasks_cancelled;
    let ready_bytes = progress
        .bytes_downloaded
        .saturating_add(progress.bytes_skipped);
    progress_bar.set_message(format!(
        "{finished_tasks}/{} accepted | {} ready",
        progress.tasks_accepted,
        HumanBytes(ready_bytes)
    ));

    match progress.expected_bytes {
        Some(expected_bytes) if expected_bytes > 0 => {
            progress_bar.set_length(expected_bytes);
            progress_bar.set_position(
                progress
                    .bytes_downloaded
                    .saturating_add(progress.bytes_skipped),
            );
        }
        _ => {
            progress_bar.set_length(progress.tasks_accepted as u64);
            progress_bar.set_position(finished_tasks as u64);
        }
    }
}
