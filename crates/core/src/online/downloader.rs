use dashmap::{
    DashMap,
    mapref::one::{Ref, RefMut},
};
use futures::StreamExt;
use std::{io::Result, sync::LazyLock};
use tokio::task::JoinSet;

use crate::error::unification::UnifiedResult;

pub struct ElementalDownloader {
    client: reqwest::Client,
    handler: DashMap<String, JoinSet<()>>,
}
static SHARED_DOWNLOADER: LazyLock<ElementalDownloader> = LazyLock::new(ElementalDownloader::new);

impl ElementalDownloader {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            handler: DashMap::new(),
        }
    }

    pub fn shared() -> &'static LazyLock<ElementalDownloader> {
        &SHARED_DOWNLOADER
    }

    pub fn create_task_group(&self, group: impl Into<String>) -> Option<JoinSet<()>> {
        self.handler.insert(group.into(), JoinSet::new())
    }

    pub fn remove_task_group(&self, group: impl Into<String>) {
        self.handler.remove(&group.into()).map(|(_, mut handler)| {
            handler.abort_all();
            drop(handler)
        });
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
        self.handler.get_mut(&task.group).map(|mut handler| {
            handler.value_mut().spawn(async move {
                let executer: Result<()> = async move {
                    let mut stream = client
                        .get(url.clone())
                        .send()
                        .await
                        .to_stdio()?
                        .bytes_stream();
                    let mut output = tokio::fs::File::create(path).await?;
                    let mut size = 0; // TODO Trace progress here
                    while let Some(item) = stream.next().await {
                        let data = item.to_stdio()?;
                        size += data.len();
                        tokio::io::copy(&mut data.as_ref(), &mut output).await?;
                    }
                    Ok(())
                }
                .await;

                match executer {
                    Ok(_) => (),
                    Err(error) => (), // TODO Trace error here
                }
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

impl DownloadTask {
    // usually use `version_name` as task_group_name.
    pub fn new(url: impl Into<String>, path: impl Into<String>, group: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            group: group.into(),
        }
    }
}
