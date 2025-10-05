use crate::storage::validate::async_file_sha1;
use anyhow::{Context, Result};
use futures::{FutureExt, StreamExt};
use reqwest::header::HeaderMap;
use scc::{HashMap, hash_map::OccupiedEntry};
use std::sync::{Arc, Weak};
use tokio::{
    io::AsyncWriteExt,
    sync::Semaphore,
    task::{JoinHandle, JoinSet, unconstrained},
};

#[derive(Debug)]
pub struct ElementalDownloader {
    client: reqwest::Client,
    handler: HashMap<String, JoinSet<()>>, // Group name : JoinSet()
    pub tracker: Arc<ElementalTaskTracker>,
    connections: Arc<Semaphore>, //TODO It should be configurable
    me: Weak<Self>,
}

#[derive(Debug)]
pub struct ElementalTaskTracker {
    pub tasks: HashMap<String, HashMap<DownloadTask, TrackedInfo>>, // Group: {task: info}
    pub bps: HashMap<String, DownloadBytesPerSecond>,
    counter: HashMap<String, JoinHandle<()>>,
    downloader: Weak<ElementalDownloader>, // to avoid cyclic strong reference
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
            tasks: HashMap::new(),
            bps: HashMap::new(),
            counter: HashMap::new(),
            downloader: downloader,
        }
    }

    pub async fn track_task(&self, task: &DownloadTask) {
        if let Some(tasks) = self.tasks.get_async(&task.group).await {
            tasks.upsert_async(task.clone(), task.track()).await;
        }
    }

    pub async fn active_task(&self, task: &DownloadTask) {
        if let Some(tasks) = self.tasks.get_async(&task.group).await {
            if let Some(mut e) = tasks.get_async(task).await {
                e.status = TrackedTaskStatus::ACTIVE;
            }
        }
    }

    //FIXME Gracefully handle the upsert case.
    pub async fn create_track_group(&self, group: impl Into<String>) -> Result<()> {
        let group = group.into();
        self.bps
            .upsert_async(group.clone(), DownloadBytesPerSecond::default())
            .await;
        let group_moved = group.clone();
        let downloader = self
            .downloader
            .upgrade()
            .context("unexpected downloader drop")?;
        self.counter
            .upsert_async(
                group.clone(),
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        if let Some(mut bps) = downloader.tracker.bps.get_async(&group_moved).await
                        {
                            bps.value = bps.count;
                            bps.count = 0;
                        } else {
                            break;
                        }
                    }
                }),
            )
            .await;
        self.tasks.upsert_async(group, HashMap::new()).await;
        Ok(())
    }

    pub async fn remove_track_group(&self, group: impl Into<String>) {
        let group = group.into();
        self.bps.remove_async(&group).await;
        self.counter.remove_async(&group).await.map(|(_, handle)| {
            handle.abort();
        });
        self.tasks.remove_async(&group).await;
    }

    pub async fn has_track_group(&self, group: impl Into<String>) -> bool {
        self.tasks.contains_async(&group.into()).await
    }
}

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

    pub async fn create_task_group(&self, group: impl Into<String>) -> Result<()> {
        let group = group.into();
        self.tracker.create_track_group(&group).await?;
        self.handler.upsert_async(group, JoinSet::new()).await;
        Ok(())
    }

    pub async fn remove_task_group(&self, group: impl Into<String>) {
        let group = group.into();
        self.handler
            .remove_async(&group)
            .await
            .map(|(_, mut handler)| {
                handler.abort_all();
                drop(handler)
            });
        self.tracker.remove_track_group(&group).await;
    }

    pub async fn has_task_group(&self, group: impl Into<String>) -> bool {
        self.handler.contains_async(&group.into()).await
    }

    pub async fn get_task_group(
        &self,
        group: impl Into<String>,
    ) -> Option<OccupiedEntry<'_, String, JoinSet<()>>> {
        self.handler.get_async(&group.into()).await
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

        if let Some(mut handler) = self.handler.get_async(&task.group).await {
            // Initialize the task tracking
            tracker.track_task(&task).await;
            let semaphore = downloader.connections.clone();
            let task_cloned = task.clone();
            let group_cloned = group.clone();
            let tracker_cloned = tracker.clone();
            handler.spawn(async move {
                let _permit = semaphore.acquire_owned().await.expect("semaphore closed");
                tracker_cloned.active_task(&task).await;

                let executer: Result<()> = async move {
                    let mut stream = client
                        .get(url.clone())
                        .headers(headers.unwrap_or_default())
                        .send()
                        .await?
                        .bytes_stream();
                    let mut output = tokio::fs::File::create(path).await?;

                    while let Some(item) = stream.next().await {
                        let data = item?;

                        tracker.tasks.get_async(&group_cloned).await.map(|tasks| {
                            tasks.get_sync(&task.clone()).map(|mut tracked| {
                                tracked.recv += data.len();
                            })
                        });

                        downloader
                            .tracker
                            .bps
                            .get_async(&group_cloned)
                            .await
                            .map(|mut bps| {
                                bps.count += data.len();
                            });
                        output.write_all(&data).await?;
                    }
                    Ok(())
                }
                .await;

                // release the semaphore permit
                drop(_permit);

                match executer {
                    Ok(_) => {
                        if let Some(tasks) = tracker_cloned.tasks.get_async(&group).await {
                            tasks.remove_async(&task_cloned).await;
                        }
                    }
                    Err(error) => {
                        if let Some(tasks) = tracker_cloned.tasks.get_async(&group).await {
                            if let Some(mut tracked) = tasks.get_async(&task_cloned).await {
                                tracked.status = TrackedTaskStatus::ERR(error.to_string());
                            }
                        }
                    }
                };
            });
        };

        Ok(())
    }

    pub async fn add_tasks(&self, tasks: Vec<DownloadTask>) -> Result<()> {
        for task in tasks {
            self.add_task(task).await?;
        }
        Ok(())
    }

    /// waiting will also cleanup joinset & waiting deque.
    pub async fn wait_group_tasks_unconstrained(&self, group: impl Into<String>) {
        let group = group.into();
        // wait for all tasks in the group to finish
        if let Some(mut tasks) = self.get_task_group(group).await {
            while let Some(Some(_)) = unconstrained(tasks.join_next()).now_or_never() {}
        }
    }

    pub async fn wait_group_tasks(&self, group: impl Into<String>) {
        let group = group.into();

        // wait for all tasks in the group to finish
        if let Some(mut tasks) = self.get_task_group(group).await {
            while let Some(_) = tasks.join_next().await {}
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
}

#[tokio::test]
async fn test_downloader() {
    let downloader = ElementalDownloader::new();
    let group_name = "test";
    println!("Creating group: {}", group_name);
    downloader.create_task_group(group_name).await.unwrap();
    let task = DownloadTask::new(
        "https://example.com/file1.txt",
        "file1.txt",
        group_name,
        None,
        None,
    );
    downloader.add_task(task).await.unwrap();
    println!("group: {:?}", downloader.tracker.tasks);

    downloader.wait_group_tasks(group_name).await;
    println!("group: {:?}", downloader.tracker.tasks);
}
