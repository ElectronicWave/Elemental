use crate::storage::validate::async_file_sha1;
use anyhow::{Context, Result, bail};
use futures::StreamExt;
use reqwest::{ClientBuilder, header::HeaderMap, retry};
use scc::{HashMap, hash_map::OccupiedEntry};
use std::{
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::{
    fs::create_dir_all,
    io::{AsyncWriteExt, BufWriter},
    sync::Semaphore,
    task::JoinHandle,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

type TaskId = String;
type GroupId = String;

#[derive(Debug)]
pub struct ElementalDownloader {
    client: reqwest::Client,
    handler: HashMap<GroupId, TaskTracker>, // Group name : TaskTracker
    pub tracker: Arc<ElementalTaskTracker>,
    connections: Arc<Semaphore>,
    me: Weak<Self>,
}

#[derive(Debug)]
pub struct ElementalTaskTracker {
    pub groups: HashMap<GroupId, GroupState>,
    downloader: Weak<ElementalDownloader>,
}

#[derive(Debug)]
pub struct GroupState {
    pub tasks: HashMap<TaskId, TrackedInfo>,
    pub bps: DownloadBytesPerSecond,
    pub token: CancellationToken,
    pub counter: JoinHandle<()>,
}

#[derive(Debug)]
pub struct ElementalDownloaderConfig {
    pub max_connections: usize,
    pub connect_timeout: Duration,
    pub retry_times: u32,
}

impl Default for ElementalDownloaderConfig {
    fn default() -> Self {
        Self {
            max_connections: 8,
            connect_timeout: Duration::from_secs(10),
            retry_times: 3,
        }
    }
}

impl GroupState {
    pub fn new(group: impl Into<String>, downloader: Arc<ElementalDownloader>) -> Self {
        let group = group.into();
        Self {
            tasks: HashMap::new(),
            bps: DownloadBytesPerSecond::default(),
            token: CancellationToken::new(),
            counter: tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    if let Some(mut state) = downloader.tracker.groups.get_async(&group).await {
                        state.bps.value = state.bps.count;
                        state.bps.count = 0;
                    } else {
                        // Task group removed, exit the loop, it will release the JoinHandle
                        break;
                    }
                }
            }),
        }
    }
}

#[derive(Debug)]
pub struct DownloadBytesPerSecond {
    pub count: usize,
    pub value: usize,
}

impl Default for DownloadBytesPerSecond {
    fn default() -> Self {
        Self { count: 0, value: 0 }
    }
}

impl ElementalTaskTracker {
    pub fn new(downloader: Weak<ElementalDownloader>) -> Self {
        Self {
            groups: HashMap::new(),
            downloader: downloader,
        }
    }

    pub async fn create_task(&self, task: &DownloadTask) {
        if let Some(state) = self.groups.get_async(&task.group).await {
            state.tasks.upsert_async(task.taskid(), task.track()).await;
        }
    }

    /// It wont create the task if not exists
    pub async fn update_task(&self, task: &DownloadTask, status: TrackedTaskStatus) {
        if let Some(state) = self.groups.get_async(&task.group).await {
            if let Some(mut e) = state.tasks.get_async(&task.taskid()).await {
                e.status = status;
            }
        }
    }

    pub async fn create_group(&self, group: impl Into<String>) -> Result<()> {
        let group = group.into();

        let downloader = self
            .downloader
            .upgrade()
            .context("unexpected downloader drop")?;

        let popout = self
            .groups
            .upsert_async(group.clone(), GroupState::new(group, downloader))
            .await;

        // Gracefully cancel the old task group if exists
        if let Some(old) = popout {
            old.token.cancel();
            old.counter.abort();
            // Wait for the task to finish
            let _ = old.counter.await;
        }

        Ok(())
    }

    pub async fn cancel_group(&self, group: impl Into<String>) {
        let group = group.into();
        if let Some(state) = self.groups.get_async(&group).await {
            state.token.cancel();
        }
    }

    /// will try cancel the task group if exists
    pub async fn remove_group(&self, group: impl Into<String>) {
        let group = group.into();
        if let Some(state) = self.groups.get_async(&group).await {
            state.token.cancel();
        }
        self.groups.remove_async(&group).await;
    }

    pub async fn has_track_group(&self, group: impl Into<String>) -> bool {
        self.groups.contains_async(&group.into()).await
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct AnyHost;
impl PartialEq<&str> for AnyHost {
    fn eq(&self, _: &&str) -> bool {
        true
    }
}
const ANY_HOST: AnyHost = AnyHost {};

impl ElementalDownloader {
    pub fn new() -> Arc<Self> {
        Arc::new_cyclic(|me| Self {
            client: reqwest::Client::new(),
            handler: HashMap::new(),
            tracker: Arc::new(ElementalTaskTracker::new(me.clone())),
            connections: Arc::new(Semaphore::new(8)), // Default max connections
            me: me.clone(),
        })
    }

    pub fn with_config_default() -> Result<Arc<Self>> {
        Self::with_config(ElementalDownloaderConfig::default())
    }

    pub fn with_config(config: ElementalDownloaderConfig) -> Result<Arc<Self>> {
        let retry_policy = retry::for_host(ANY_HOST)
            .max_retries_per_request(config.retry_times)
            .classify_fn(|req_rep| {
                if req_rep.error().is_some() {
                    req_rep.retryable()
                } else if matches!(req_rep.status(), Some(status) if status.is_server_error()) {
                    req_rep.retryable()
                } else {
                    req_rep.success()
                }
            });
        let client = ClientBuilder::new()
            .retry(retry_policy)
            .connect_timeout(config.connect_timeout)
            .build()?;

        Ok(Arc::new_cyclic(|me| Self {
            client,
            handler: HashMap::new(),
            tracker: Arc::new(ElementalTaskTracker::new(me.clone())),
            connections: Arc::new(Semaphore::new(config.max_connections)),
            me: me.clone(),
        }))
    }

    pub async fn create_group(&self, group: impl Into<String>) -> Result<()> {
        let group = group.into();
        self.tracker.create_group(&group).await?;
        self.handler.upsert_async(group, TaskTracker::new()).await;
        Ok(())
    }

    pub async fn remove_group(&self, group: impl Into<String>) {
        let group = group.into();
        self.handler.remove_async(&group).await.map(|(_, handler)| {
            handler.close();
        });
        self.tracker.remove_group(&group).await;
    }

    pub async fn has_group(&self, group: impl Into<String>) -> bool {
        self.handler.contains_async(&group.into()).await
    }

    pub async fn get_group(
        &self,
        group: impl Into<String>,
    ) -> Option<OccupiedEntry<'_, String, TaskTracker>> {
        self.handler.get_async(&group.into()).await
    }

    pub async fn get_group_state(
        &self,
        group: impl Into<String>,
    ) -> Option<OccupiedEntry<'_, GroupId, GroupState>> {
        self.tracker.groups.get_async(&group.into()).await
    }

    pub async fn add_task(&self, task: DownloadTask) -> Result<()> {
        self.add_task_with_headers(task, None).await
    }

    pub async fn add_task_with_headers(
        &self,
        task: DownloadTask,
        headers: Option<HeaderMap>,
    ) -> Result<()> {
        // validate file exist
        if let Some(sha1) = &task.sha1 {
            if async_file_sha1(&task.path)
                .await
                .map_or(false, |hash| hash == *sha1)
            {
                return Ok(());
            }
        }

        let client = self.client.clone();
        let downloader = self.me.upgrade().context("unexpected downloader drop")?;
        let tracker = self.tracker.clone();

        let url = task.url.clone();
        let path = task.path.clone();
        let group = task.group.clone();
        let headers = headers.unwrap_or_default();
        let token = self
            .tracker
            .groups
            .get_async(&task.group)
            .await
            .context("task group not found")?
            .token
            .child_token();

        if let Some(handler) = self.handler.get_async(&task.group).await {
            // Initialize the task tracking
            tracker.create_task(&task).await;
            let semaphore = downloader.connections.clone();
            let task_cloned = task.clone();
            let taskid = task.taskid();
            let group_cloned = group.clone();
            let tracker_cloned = tracker.clone();

            handler.spawn(async move {
                let executer =
                    async move {
                        let mut stream = client
                            .get(url.clone())
                            .headers(headers)
                            .send()
                            .await?
                            .error_for_status()?
                            .bytes_stream();

                        let path = std::path::Path::new(&path);
                        create_dir_all(path.parent().context("Can't get parent directory")?)
                            .await?;
                        let file = tokio::fs::File::create(path).await?;
                        let mut output = BufWriter::with_capacity(128 * 1024, file);
                        while let Some(item) = stream.next().await {
                            let data = item?;

                            tracker.groups.get_async(&group_cloned).await.map(|state| {
                                state.tasks.get_sync(&task.taskid()).map(|mut tracked| {
                                    tracked.recv += data.len();
                                })
                            });

                            downloader.tracker.groups.get_async(&group_cloned).await.map(
                                |mut state| {
                                    state.bps.count += data.len();
                                },
                            );
                            output.write_all(&data).await?;
                        }
                        output.flush().await?;
                        anyhow::Ok(())
                    };
                let path_cloned = task_cloned.path.clone();
                let cleanup = async move {
                    //TODO ignore the error?
                    let _ = tokio::fs::remove_file(&path_cloned).await;
                };
                let _permit = semaphore.acquire_owned().await.expect("semaphore closed");
                tokio::select! {
                    result = executer => {
                        match result {
                            Ok(_) => {
                                if let Some(state) = tracker_cloned.groups.get_async(&group).await {
                                    state.tasks.remove_async(&taskid).await;
                                }
                            }
                            Err(error) => {
                                if let Some(state) = tracker_cloned.groups.get_async(&group).await {
                                    if let Some(mut tracked) = state.tasks.get_async(&taskid).await {
                                        tracked.status = TrackedTaskStatus::ERR(error.to_string());
                                    }
                                }
                            }
                        };
                    },
                    () = token.cancelled() => {
                        cleanup.await;
                    },
                }

                // release the semaphore permit
                drop(_permit);
            });
        } else {
            bail!("task group '{}' not found", task.group);
        }

        Ok(())
    }

    pub async fn add_tasks(&self, tasks: Vec<DownloadTask>) -> Result<()> {
        for task in tasks {
            self.add_task(task).await?;
        }
        Ok(())
    }

    pub async fn wait_group_empty(&self, group: impl Into<String>) {
        let group = group.into();
        if let Some(tasks) = self.get_group(group).await {
            while !tasks.is_empty() {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }

    pub async fn wait_group(&self, group: impl Into<String>) {
        let group = group.into();

        // wait for all tasks in the group to finish
        if let Some(tasks) = self.get_group(group).await {
            tasks.wait().await;
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DownloadTask {
    pub url: String,
    pub path: String,
    pub group: String,
    pub total: Option<usize>,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TrackedInfo {
    pub recv: usize,
    pub status: TrackedTaskStatus,
}

#[derive(Debug, Clone)]
pub enum TrackedTaskStatus {
    ERR(String),
    ACTIVE,
    WAITING,
}

impl DownloadTask {
    // usually use `version_name` as task_group_name.
    pub fn new(
        url: impl Into<String>,
        path: impl Into<String>,
        group: impl Into<String>,
        total: Option<usize>,
        sha1: Option<String>,
    ) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            group: group.into(),
            total,
            sha1,
        }
    }

    pub fn track(&self) -> TrackedInfo {
        TrackedInfo {
            recv: 0,
            status: TrackedTaskStatus::WAITING,
        }
    }

    pub fn taskid(&self) -> TaskId {
        format!("{}-{}-{}", self.group, self.url, self.path)
    }
}

#[tokio::test]
async fn test_downloader() {
    let downloader = ElementalDownloader::with_config_default().unwrap();
    let group_name = "test";
    println!("Creating group: {}", group_name);
    downloader.create_group(group_name).await.unwrap();
    let task = DownloadTask::new(
        "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",
        "versions.json",
        group_name,
        None,
        None,
    );
    downloader.add_task(task).await.unwrap();
    println!("group: {:?}", downloader.tracker.groups);

    downloader.wait_group_empty(group_name).await;
    downloader.remove_group(group_name).await;
    println!("group: {:?}", downloader.tracker.groups);
}
