use std::{collections::BTreeSet, future::Future, path::Path, time::Duration};

use anyhow::{Context, Result};
use elemental::{
    infra::downloader::{ActiveTask, DownloadProgress, DownloadSession},
    launcher::PrepareStage,
};
use indicatif::{HumanBytes, HumanDuration, ProgressBar, ProgressStyle};

const SPINNER_TEMPLATE: &str = "{spinner:.green} {msg}";
const DOWNLOAD_TEMPLATE: &str = "{spinner:.green} {prefix} [{bar:40.cyan/blue}]\n{msg}";
const VISIBLE_ACTIVE_TASKS: usize = 4;

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

pub async fn download(session: &DownloadSession) -> Result<DownloadProgressBar> {
    let style = ProgressStyle::with_template(DOWNLOAD_TEMPLATE)
        .context("invalid download progress template")?;
    let progress_bar = ProgressBar::new(0)
        .with_style(style)
        .with_message("  waiting for tasks");
    progress_bar.enable_steady_tick(Duration::from_millis(100));

    let mut progress_rx = session.progress_receiver();
    let mut progress = session.progress();
    let (stage_tx, mut stage_rx) = tokio::sync::watch::channel(PrepareStage::ResolvingMetadata);
    let mut stage = PrepareStage::ResolvingMetadata;
    apply_prepare_progress(&progress_bar, &stage, &progress);

    let task_progress_bar = progress_bar.clone();
    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                changed = progress_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    progress = progress_rx.borrow_and_update().clone();
                }
                changed = stage_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    stage = stage_rx.borrow_and_update().clone();
                }
            }
            apply_prepare_progress(&task_progress_bar, &stage, &progress);
        }
    });

    Ok(DownloadProgressBar {
        progress_bar,
        stage_tx,
        task,
    })
}

pub struct DownloadProgressBar {
    progress_bar: ProgressBar,
    stage_tx: tokio::sync::watch::Sender<PrepareStage>,
    task: tokio::task::JoinHandle<()>,
}

impl DownloadProgressBar {
    pub fn stage_observer(&self) -> impl Fn(PrepareStage) + Send + Sync + 'static {
        let stage_tx = self.stage_tx.clone();
        move |stage| {
            let _ = stage_tx.send(stage);
        }
    }
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

fn apply_prepare_progress(
    progress_bar: &ProgressBar,
    stage: &PrepareStage,
    progress: &DownloadProgress,
) {
    if *stage == PrepareStage::Downloading || !progress.active_tasks.is_empty() {
        apply_download_progress(progress_bar, progress);
        return;
    }

    match stage {
        PrepareStage::ResolvingMetadata => {
            progress_bar.set_prefix("Resolving metadata");
            progress_bar.set_message("  fetching manifests");
            progress_bar.set_length(1);
            progress_bar.set_position(0);
        }
        PrepareStage::Downloading => {
            apply_download_progress(progress_bar, progress);
        }
        PrepareStage::ExtractingNatives => {
            progress_bar.set_prefix("Extracting natives");
            progress_bar.set_message("  unpacking native libraries");
            progress_bar.set_length(1);
            progress_bar.set_position(0);
        }
        PrepareStage::RunningProcessor {
            family_name,
            index,
            total,
            jar,
        } => {
            progress_bar.set_prefix(format!("Running {family_name} processor ({index}/{total})"));
            progress_bar.set_message(format!("  {jar}"));
            progress_bar.set_length((*total).max(1) as u64);
            progress_bar.set_position((*index).min(*total) as u64);
        }
        PrepareStage::Ready => {
            progress_bar.set_prefix("Ready");
            progress_bar.set_message(String::new());
            progress_bar.set_length(1);
            progress_bar.set_position(1);
        }
    }
}

fn apply_download_progress(progress_bar: &ProgressBar, progress: &DownloadProgress) {
    let phase = display_phase(progress);
    progress_bar.set_prefix(format!(
        "{phase} | {finished}/{accepted} tasks | {ready} | {speed} | ETA {eta}",
        phase = phase,
        finished = progress.tasks_finished(),
        accepted = progress.tasks_accepted,
        ready = display_ready(progress),
        speed = display_speed(progress.bytes_per_second),
        eta = display_eta(progress.eta),
    ));
    progress_bar.set_message(display_active_tasks(progress));

    match progress.expected_bytes {
        Some(expected_bytes) if expected_bytes > 0 => {
            progress_bar.set_length(expected_bytes);
            progress_bar.set_position(progress.ready_bytes().min(expected_bytes));
        }
        _ => {
            progress_bar.set_length(progress.tasks_accepted as u64);
            progress_bar.set_position(progress.tasks_finished() as u64);
        }
    }
}

fn display_ready(progress: &DownloadProgress) -> String {
    match progress.expected_bytes {
        Some(expected_bytes) if expected_bytes > 0 => format!(
            "{} / {}",
            HumanBytes(progress.ready_bytes()),
            HumanBytes(expected_bytes)
        ),
        _ => format!("{} ready", HumanBytes(progress.ready_bytes())),
    }
}

fn display_speed(bytes_per_second: u64) -> String {
    if bytes_per_second == 0 {
        "--".to_owned()
    } else {
        format!("{}/s", HumanBytes(bytes_per_second))
    }
}

fn display_eta(eta: Option<Duration>) -> String {
    eta.map(|eta| format!("{:#}", HumanDuration(eta)))
        .unwrap_or_else(|| "--".to_owned())
}

fn display_active_tasks(progress: &DownloadProgress) -> String {
    if progress.active_tasks.is_empty() {
        return "  waiting for tasks".to_owned();
    }

    let mut lines = progress
        .active_tasks
        .iter()
        .take(VISIBLE_ACTIVE_TASKS)
        .map(|task| format!("  {}", display_task(task)))
        .collect::<Vec<String>>();
    let hidden = progress
        .active_tasks
        .len()
        .saturating_sub(VISIBLE_ACTIVE_TASKS);
    if hidden > 0 {
        lines.push(format!("  ... +{hidden} hidden"));
    }

    lines.join("\n")
}

fn display_task(task: &ActiveTask) -> String {
    let Some(file_name) = task.path.file_name() else {
        return task.path.display().to_string();
    };
    let name = file_name.to_string_lossy();
    let Some(parent) = task.path.parent().and_then(Path::file_name) else {
        return name.into_owned();
    };

    format!("{}/{}", parent.to_string_lossy(), name)
}

fn display_phase(progress: &DownloadProgress) -> String {
    let groups = progress
        .active_tasks
        .iter()
        .filter_map(|task| task.group.as_deref())
        .collect::<BTreeSet<&str>>();
    if !groups.is_empty() {
        return groups
            .iter()
            .map(|group| display_group(group))
            .collect::<Vec<String>>()
            .join(", ");
    }

    progress
        .last_group
        .as_deref()
        .map(display_group)
        .unwrap_or_else(|| "Preparing".to_owned())
}

fn display_group(group: &str) -> String {
    let mut label = group.replace('-', " ");
    if let Some(first) = label.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    label
}
