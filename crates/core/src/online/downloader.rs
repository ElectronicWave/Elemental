use crate::storage::validate::file_sha1;
use anyhow::Result;
use dashmap::{
    DashMap,
    mapref::one::{Ref, RefMut},
};
use futures::{FutureExt, StreamExt};
use reqwest::header::HeaderMap;
use std::{
    collections::VecDeque,
    sync::{Arc, LazyLock},
};
use tokio::{
    io::AsyncWriteExt,
    sync::Semaphore,
    task::{JoinHandle, JoinSet, unconstrained},
};
// Please use `ElementalDownloader::shared()` to access the shared downloader instance.
pub struct ElementalDownloader {
    client: reqwest::Client,
    handler: DashMap<String, JoinSet<()>>, // Group name : JoinSet()
    waiting: DashMap<String, VecDeque<DownloadTask>>,
    pub tracker: ElementalTaskTracker,
    connections: Arc<Semaphore>,
}

#[derive(Debug)]
pub struct ElementalTaskTracker {
    pub tasks: DashMap<String, DashMap<DownloadTask, TrackedInfo>>, // Group: {task: info}
    pub bps: DashMap<String, DownloadBytesPerSecond>,
    counter: DashMap<String, JoinHandle<()>>,
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
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
            bps: DashMap::new(),
            counter: DashMap::new(),
        }
    }

    pub fn track_task(&self, task: &DownloadTask) {
        self.tasks
            .get(&task.group)
            .map(|tasks| tasks.insert(task.clone(), task.track()));
    }

    pub fn active_task(&self, task: &DownloadTask) {
        self.tasks.get(&task.group).map(|tasks| {
            tasks.get_mut(task).map(|mut tracked| {
                tracked.value_mut().status = TrackedTaskStatus::ACTIVE;
            })
        });
    }

    pub fn create_track_group(&self, group: impl Into<String>) {
        let group = group.into();
        self.bps
            .insert(group.clone(), DownloadBytesPerSecond::default());
        let group_moved = group.clone();
        self.counter.insert(
            group.clone(),
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    if let Some(mut bps) = SHARED_DOWNLOADER.tracker.bps.get_mut(&group_moved) {
                        bps.value_mut().value = bps.value().count;
                        bps.value_mut().count = 0;
                    } else {
                        break;
                    }
                }
            }),
        );
        self.tasks.insert(group, DashMap::new());
    }

    pub fn remove_track_group(&self, group: impl Into<String>) {
        let group = group.into();
        self.bps.remove(&group);
        self.counter.remove(&group).map(|(_, handle)| {
            handle.abort();
        });
        self.tasks.remove(&group);
    }

    pub fn has_track_group(&self, group: impl Into<String>) -> bool {
        self.tasks.contains_key(&group.into())
    }
}

static SHARED_DOWNLOADER: LazyLock<ElementalDownloader> = LazyLock::new(ElementalDownloader::new);

impl ElementalDownloader {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            handler: DashMap::new(),
            waiting: DashMap::new(),
            tracker: ElementalTaskTracker::new(),
            connections: Arc::new(Semaphore::new(8)), // Default max connections
        }
    }

    pub fn shared() -> &'static LazyLock<ElementalDownloader> {
        &SHARED_DOWNLOADER
    }

    pub fn create_task_group(&self, group: impl Into<String>) -> Option<JoinSet<()>> {
        let group = group.into();
        self.tracker.create_track_group(&group);
        self.waiting.insert(group.clone(), VecDeque::new());
        self.handler.insert(group, JoinSet::new())
    }

    pub fn remove_task_group(&self, group: impl Into<String>) {
        let group = group.into();
        self.handler.remove(&group).map(|(_, mut handler)| {
            handler.abort_all();
            drop(handler)
        });
        self.waiting.remove(&group);
        self.tracker.remove_track_group(&group);
    }

    pub fn has_task_group(&self, group: impl Into<String>) -> bool {
        self.handler.contains_key(&group.into())
    }

    pub fn get_task_group(&self, group: impl Into<String>) -> Option<Ref<'_, String, JoinSet<()>>> {
        self.handler.get(&group.into())
    }

    pub async fn group_context<F: Future<Output = Result<()>> + Send + 'static>(
        &self,
        group: impl Into<String>,
        future: fn(downloader: &'static LazyLock<ElementalDownloader>, group: String) -> F,
    ) -> Result<()> {
        let group = group.into();
        self.create_task_group(group.clone());
        future(&SHARED_DOWNLOADER, group.clone()).await?;
        self.remove_task_group(group);
        Ok(())
    }

    pub fn get_task_group_mut(
        &self,
        group: impl Into<String>,
    ) -> Option<RefMut<'_, String, JoinSet<()>>> {
        self.handler.get_mut(&group.into())
    }

    pub fn add_task(&self, task: DownloadTask) {
        self.add_task_with_headers(task, None);
    }

    pub fn add_task_with_headers(&self, task: DownloadTask, headers: Option<HeaderMap>) {
        // validate file exist
        if let Some(sha1) = &task.sha1 {
            if file_sha1(&task.path).map_or(false, |hash| hash == *sha1) {
                return;
            }
        }

        let client = self.client.clone();
        let url = task.url.clone();
        let path = task.path.clone();
        let group = task.group.clone();

        self.handler.get_mut(&task.group).map(|mut handler| {
            // Initialize the task tracking
            SHARED_DOWNLOADER.tracker.track_task(&task);
            let semaphore = SHARED_DOWNLOADER.connections.clone();
            let task_cloned = task.clone();
            let group_cloned = group.clone();
            handler.value_mut().spawn(async move {
                println!("Waiting for semaphore: {:?} in group: {}", task, group);
                let _permit = semaphore.acquire_owned().await.expect("semaphore closed");
                println!("Started task: {:?} in group: {}", task, group);
                let tracker = &SHARED_DOWNLOADER.tracker;
                tracker.active_task(&task);
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

                        tracker.tasks.get_mut(&group_cloned).map(|mut tasks| {
                            tasks.value_mut().get_mut(&task.clone()).map(|mut tracked| {
                                tracked.recv += data.len();
                            })
                        });

                        SHARED_DOWNLOADER
                            .tracker
                            .bps
                            .get_mut(&group_cloned)
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

                // active another task if available
                if let Some(mut waiting) = SHARED_DOWNLOADER.waiting.get_mut(&task_cloned.group) {
                    if let Some(task) = waiting.pop_front() {
                        SHARED_DOWNLOADER.add_task(task);
                    }
                }

                match executer {
                    Ok(_) => SHARED_DOWNLOADER
                        .tracker
                        .tasks
                        .get_mut(&group)
                        .map(|mut tasks| {
                            // Remove this task to clean up
                            tasks.value_mut().remove(&task_cloned);
                            Some(())
                        }),
                    Err(error) => {
                        SHARED_DOWNLOADER
                            .tracker
                            .tasks
                            .get_mut(&group)
                            .map(|mut tasks| {
                                tasks.value_mut().get_mut(&task_cloned).map(|mut tracked| {
                                    tracked.value_mut().status =
                                        TrackedTaskStatus::ERR(error.to_string());
                                })
                            })
                    }
                };
            });
        });
    }

    pub fn add_tasks(&self, tasks: Vec<DownloadTask>) {
        for task in tasks {
            self.add_task(task);
        }
    }

    /// waiting will also cleanup joinset & waiting deque.
    pub async fn wait_group_tasks_unconstrained(&self, group: impl Into<String>) {
        let group = group.into();
        // ensure all tasks are not in waiting state
        while let Some(mut waiting) = self.waiting.get_mut(&group) {
            if waiting.is_empty() {
                // Clean up it self
                waiting.value_mut().shrink_to_fit();
                break;
            }
        }

        // wait for all tasks in the group to finish
        if let Some(mut tasks) = self.get_task_group_mut(group) {
            while let Some(Some(_)) = unconstrained(tasks.value_mut().join_next()).now_or_never() {}
        }
    }

    pub async fn wait_group_tasks(&self, group: impl Into<String>) {
        let group = group.into();
        // ensure all tasks are not in waiting state
        while let Some(mut waiting) = self.waiting.get_mut(&group) {
            if waiting.is_empty() {
                // Clean up it self
                waiting.value_mut().shrink_to_fit();
                break;
            }
        }

        // wait for all tasks in the group to finish
        if let Some(mut tasks) = self.get_task_group_mut(group) {
            while let Some(_) = tasks.value_mut().join_next().await {}
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
    let downloader = ElementalDownloader::shared();
    let group_name = "test";

    let result = downloader
        .group_context(group_name, |downloader, group| async {
            let task = DownloadTask::new(
                "https://example.com/file1.txt",
                "file1.txt",
                &group,
                None,
                None,
            );
            downloader.add_task(task);
            downloader.wait_group_tasks(group).await;
            Ok(())
        })
        .await;
    println!("{:?}", result);
    println!("group: {:?}", downloader.tracker.tasks);
}
