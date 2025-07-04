use dashmap::{
    DashMap,
    mapref::one::{Ref, RefMut},
};
use futures::StreamExt;
use std::{
    io::Result,
    sync::{Arc, LazyLock},
    time::SystemTime,
    vec,
};
use tokio::task::{JoinError, JoinSet};

use crate::error::unification::UnifiedResult;

pub struct ElementalDownloader {
    client: reqwest::Client,
    handler: DashMap<String, JoinSet<()>>, // Group name : JoinSet()
    pub tracker: Arc<ElementalTaskTracker>,
}
#[derive(Debug)]
pub struct ElementalTaskTracker {
    pub tasks: DashMap<String, DashMap<DownloadTask, TrackedInfo>>, // Group: {task: info}
    pub bps: DashMap<String, DownloadBitsPerSecond>,
}
#[derive(Debug)]
pub struct DownloadBitsPerSecond {
    pub lasttime: SystemTime,
    pub counter: usize,
    pub bps: usize,
}
impl Default for DownloadBitsPerSecond {
    fn default() -> Self {
        Self {
            lasttime: SystemTime::now(),
            counter: 0,
            bps: 0,
        }
    }
}

impl ElementalTaskTracker {
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
            bps: DashMap::new(),
        }
    }

    pub fn track_task(&self, task: &DownloadTask) {
        self.tasks
            .get_mut(&task.group)
            .map(|mut tasks| tasks.value_mut().insert(task.clone(), task.track()));
    }

    pub fn create_track_group(&self, group: impl Into<String>) {
        let group = group.into();
        self.bps
            .insert(group.clone(), DownloadBitsPerSecond::default());
        self.tasks.insert(group.into(), DashMap::new());
    }

    pub fn remove_track_group(&self, group: impl Into<String>) {
        let group = group.into();
        self.bps.remove(&group);
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
            tracker: Arc::new(ElementalTaskTracker::new()),
        }
    }

    pub fn shared() -> &'static LazyLock<ElementalDownloader> {
        &SHARED_DOWNLOADER
    }

    pub fn create_task_group(&self, group: impl Into<String>) -> Option<JoinSet<()>> {
        let group = group.into();
        self.tracker.create_track_group(&group);
        self.handler.insert(group, JoinSet::new())
    }

    pub fn remove_task_group(&self, group: impl Into<String>) {
        let group = group.into();
        self.handler.remove(&group).map(|(_, mut handler)| {
            handler.abort_all();
            drop(handler)
        });
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
        let tracker = self.tracker.clone();

        self.handler.get_mut(&task.group).map(|mut handler| {
            tracker.track_task(&task);
            let task_cloned = task.clone();
            let group_cloned = group.clone();
            let tracker_cloned = tracker.clone();

            handler.value_mut().spawn(async move {
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

                        tracker_cloned
                            .tasks
                            .get_mut(&group_cloned)
                            .map(|mut tasks| {
                                tasks.value_mut().get_mut(&task.clone()).map(|mut tracked| {
                                    tracked.recv += data.len();
                                })
                            });

                        tracker_cloned.bps.get_mut(&group_cloned).map(|mut bps| {
                            let current = SystemTime::now();
                            if current.duration_since(bps.lasttime).unwrap().as_secs() >= 1 {
                                bps.bps = bps.value_mut().counter + data.len();
                                bps.counter = 0;
                                bps.lasttime = current;
                            } else {
                                bps.counter += data.len();
                            }
                        });
                        tokio::io::copy(&mut data.as_ref(), &mut output).await?;
                    }
                    Ok(())
                }
                .await;

                match executer {
                    Ok(_) => tracker.tasks.get_mut(&group).map(|mut tasks| {
                        tasks.value_mut().get_mut(&task_cloned).map(|mut tracked| {
                            tracked.value_mut().status = TrackedTaskStatus::DONE;
                        })
                    }),
                    Err(error) => tracker.tasks.get_mut(&group).map(|mut tasks| {
                        tasks.value_mut().get_mut(&task_cloned).map(|mut tracked| {
                            tracked.value_mut().status = TrackedTaskStatus::ERR(error.to_string());
                        })
                    }),
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
        let mut result = vec![];
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TrackedInfo {
    pub recv: usize,
    pub status: TrackedTaskStatus,
    pub bps: usize,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum TrackedTaskStatus {
    ERR(String),
    ACTIVE,
    DONE,
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
            status: TrackedTaskStatus::ACTIVE,
            bps: 0,
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
