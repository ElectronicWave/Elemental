use dashmap::{
    DashMap,
    mapref::one::{Ref, RefMut},
};
use futures::StreamExt;
use std::{
    collections::VecDeque,
    io::Result,
    sync::{LazyLock, RwLock},
};
use tokio::task::{JoinError, JoinHandle, JoinSet};

use crate::error::unification::UnifiedResult;
// Please use `ElementalDownloader::shared()` to access the shared downloader instance.
pub struct ElementalDownloader {
    client: reqwest::Client,
    handler: DashMap<String, JoinSet<()>>, // Group name : JoinSet()
    waiting: DashMap<String, VecDeque<DownloadTask>>,
    pub tracker: ElementalTaskTracker, // TODO Remove Arc
    connection_count: RwLock<usize>,
    pub configs: RwLock<ElementalDownloaderConfigs>,
}

pub struct ElementalDownloaderConfigs {
    pub max_connections: usize, // Maximum number of concurrent connections, default is 8, higher will help download small files faster.
}
#[derive(Debug)]
pub struct ElementalTaskTracker {
    pub tasks: DashMap<String, DashMap<DownloadTask, TrackedInfo>>, // Group: {task: info}
    pub bps: DashMap<String, DownloadBytesPerSecond>,
    counter: DashMap<String, JoinHandle<()>>,
}
#[derive(Debug)]
pub struct DownloadBytesPerSecond {
    pub counter: usize,
    pub bps: usize,
}
impl Default for DownloadBytesPerSecond {
    fn default() -> Self {
        Self { counter: 0, bps: 0 }
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
                        bps.value_mut().bps = bps.value().counter;
                        bps.value_mut().counter = 0;
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
            connection_count: RwLock::new(0),
            configs: RwLock::new(ElementalDownloaderConfigs {
                max_connections: 8, // Default max connections
            }),
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

    pub async fn task_group_context<F: Future<Output = Result<()>> + Send + 'static>(
        &self,
        group: impl Into<String>,
        future: fn(group: String) -> F,
    ) -> Result<()> {
        let group = group.into();
        self.create_task_group(group.clone());
        future(group.clone()).await?;
        self.remove_task_group(group);
        Ok(())
    }

    pub fn get_task_group_mut(
        &self,
        group: impl Into<String>,
    ) -> Option<RefMut<'_, String, JoinSet<()>>> {
        self.handler.get_mut(&group.into())
    }

    pub fn add_task(&self, task: DownloadTask) -> Option<()> {
        let client = self.client.clone();
        let url = task.url.clone();
        let path = task.path.clone();
        let group = task.group.clone();

        self.handler.get_mut(&task.group).map(|mut handler| {
            // Initialize the task tracking
            SHARED_DOWNLOADER.tracker.track_task(&task);
            // This num should be configurable
            if self.connection_count.read().unwrap().clone()
                > self.configs.read().unwrap().max_connections.clone()
            {
                self.waiting.get_mut(&task.group).map(|mut waiting| {
                    waiting.push_back(task);
                });
                return;
            } else {
                // println!("add task: {}", task.url);
                *self.connection_count.write().unwrap() += 1;
            }

            let task_cloned = task.clone();
            let group_cloned = group.clone();

            handler.value_mut().spawn(async move {
                let tracker = &SHARED_DOWNLOADER.tracker;
                tracker.active_task(&task);
                let executer: Result<()> = async move {
                    let mut stream = client
                        .get(url.clone())
                        .send()
                        .await
                        .to_stdio()?
                        .bytes_stream();
                    let mut output = tokio::fs::File::create(path).await?;

                    while let Some(item) = stream.next().await {
                        let data = item.to_stdio()?;

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
                                bps.counter += data.len();
                            });
                        tokio::io::copy(&mut data.as_ref(), &mut output).await?;
                    }
                    Ok(())
                }
                .await;

                // shrink the connection count
                *SHARED_DOWNLOADER.connection_count.write().unwrap() -= 1;

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
                            tasks.value_mut().get_mut(&task_cloned).map(|mut tracked| {
                                tracked.value_mut().status = TrackedTaskStatus::DONE;
                            })
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
        })
    }

    pub fn add_tasks(&self, tasks: Vec<DownloadTask>) -> Vec<Option<()>> {
        tasks.into_iter().map(|task| self.add_task(task)).collect()
    }

    pub async fn wait_group_tasks(
        &self,
        group: impl Into<String>,
    ) -> Vec<core::result::Result<(), JoinError>> {
        let group = group.into();
        // ensure all tasks are not in waiting state
        while let Some(waiting) = self.waiting.get_mut(&group) {
            if waiting.is_empty() {
                println!("waiting tasks are empty.");
                break;
            }
        }

        let mut result = vec![];
        // wait for all tasks in the group to finish
        if let Some(mut tasks) = self.get_task_group_mut(group) {
            while let Some(res) = tasks.value_mut().join_next().await {
                result.push(res);
            }
        }

        result
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DownloadTask {
    pub url: String,
    pub path: String,
    pub group: String,
    pub total: Option<usize>,
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
    DONE,
    WAITING,
}

impl DownloadTask {
    // usually use `version_name` as task_group_name.
    pub fn new(
        url: impl Into<String>,
        path: impl Into<String>,
        group: impl Into<String>,
        total: Option<usize>,
    ) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            group: group.into(),
            total,
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
        .task_group_context(group_name, |group| async {
            let task = DownloadTask::new(
                "https://example.com/file1.txt",
                "file1.txt",
                "group_name",
                None,
            );
            let downloader = ElementalDownloader::shared();
            downloader.add_task(task);
            downloader.wait_group_tasks(group).await;
            Ok(())
        })
        .await;
    println!("{:?}", result);
    println!("group: {:?}", downloader.tracker.tasks);
}
