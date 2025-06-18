use dashmap::{
    DashMap,
    mapref::one::{Ref, RefMut},
};
use futures::StreamExt;
use std::{
    io::Result,
    sync::{Arc, LazyLock},
};
use tokio::task::JoinSet;

use crate::error::unification::UnifiedResult;

pub struct ElementalDownloader {
    client: reqwest::Client,
    handler: DashMap<String, JoinSet<()>>, // Group name : JoinSet()
    pub tracker: Arc<ElementalTaskTracker>,
}

pub struct ElementalTaskTracker {
    pub tasks: DashMap<String, DashMap<DownloadTask, TrackedInfo>>,
}

impl ElementalTaskTracker {
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
        }
    }

    pub fn track_task(&self, task: &DownloadTask) {
        self.tasks
            .get_mut(&task.group)
            .map(|mut tasks| tasks.value_mut().insert(task.clone(), task.track()));
    }

    pub fn create_track_group(&self, group: impl Into<String>) {
        self.tasks.insert(group.into(), DashMap::new());
    }
    pub fn remove_track_group(&self, group: impl Into<String>) {
        self.tasks.remove(&group.into());
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
        self.tracker.remove_track_group(&group);
        self.handler.remove(&group).map(|(_, mut handler)| {
            handler.abort_all();
            drop(handler)
        });
    }

    pub fn has_task_group(&self, group: impl Into<String>) -> bool {
        self.handler.contains_key(&group.into())
    }

    pub fn get_task_group(&self, group: impl Into<String>) -> Option<Ref<'_, String, JoinSet<()>>> {
        self.handler.get(&group.into())
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
                                    tracked.value_mut().recv += data.len();
                                })
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
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DownloadTask {
    pub url: String,
    pub path: String,
    pub group: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TrackedInfo {
    pub recv: usize,
    pub status: TrackedTaskStatus,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum TrackedTaskStatus {
    ERR(String),
    ACTIVE,
    DONE,
}

impl DownloadTask {
    // usually use `version_name` as task_group_name.
    pub fn new(url: impl Into<String>, path: impl Into<String>, group: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            group: group.into(),
        }
    }

    pub fn track(&self) -> TrackedInfo {
        TrackedInfo {
            recv: 0,
            status: TrackedTaskStatus::ACTIVE,
        }
    }
}

#[tokio::test]
async fn test_downloader() {
    let downloader = ElementalDownloader::shared();
    let group_name = "test";

    downloader.create_task_group(group_name);
    let task = DownloadTask::new("https://example.com/file1.txt", "file1.txt", group_name);

    downloader.add_task(task);
    // Wait for the task to complete
    if let Some(mut t) = downloader.get_task_group_mut(group_name) {
        while let Some(res) = t.value_mut().join_next().await {
            println!("{:?}", res);
        }
    }

    println!("group: {:?}", downloader.tracker.tasks);
}
