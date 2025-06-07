use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::storage::jar::JarFile;

pub struct ElementalDownloader {
    client: reqwest::Client,
    pub tracer: Arc<ElementalDownloaderTracer>,
}

pub struct ElementalDownloaderTracer {
    group_counter: Mutex<HashMap<String, usize>>,
    active_tasks: Mutex<HashSet<DownloadTask>>,
}

pub enum TaskStatus {
    OK,
    ERR(String),
    CANCEL,
}

impl ElementalDownloaderTracer {
    pub fn new() -> Self {
        Self {
            group_counter: Mutex::new(HashMap::new()),
            active_tasks: Mutex::new(HashSet::new()),
        }
    }

    pub async fn trace_task(&self, task: DownloadTask) {
        if let Some(task_group_name) = &task.task_group_name {
            let mut gurad = self.group_counter.lock().await;
            if let Some(count) = gurad.get(&task_group_name.clone()) {
                let count = count.clone();
                gurad.insert(task_group_name.clone(), count + 1);
            } else {
                gurad.insert(task_group_name.clone(), 1);
            }
            drop(gurad);
        }

        let mut gurad = self.active_tasks.lock().await;
        gurad.insert(task);
        drop(gurad);
    }

    pub async fn stop_trace_task(&self, task: DownloadTask, status: TaskStatus) {
        if let Some(task_group_name) = &task.task_group_name {
            let mut gurad = self.group_counter.lock().await;
            if let Some(count) = gurad.get(&task_group_name.clone()) {
                let count = count.clone();
                if count.clone() == 1 {
                    gurad.remove(task_group_name);
                } else {
                    gurad.insert(task_group_name.clone(), count - 1);
                }
            }
            drop(gurad);
        }

        let mut gurad = self.active_tasks.lock().await;
        gurad.remove(&task);
        drop(gurad);

        match status {
            TaskStatus::OK => match task.callback {
                DownloadTaskCallback::DEFAULT => todo!(),
                DownloadTaskCallback::NATIVELIB(src, dest, exclude) => {
                    JarFile::new(src).extract_blocking(dest).unwrap(); // TODO Remove unwrap here.
                }
                DownloadTaskCallback::NONE => (),
            },
            TaskStatus::ERR(msg) => log::error!("task err: {}", msg),
            TaskStatus::CANCEL => (),
        }
    }
}

static SHARED_DOWNLOADER: LazyLock<ElementalDownloader> = LazyLock::new(ElementalDownloader::new);

impl ElementalDownloader {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            tracer: Arc::new(ElementalDownloaderTracer::new()),
        }
    }

    pub fn shared() -> &'static LazyLock<ElementalDownloader> {
        &SHARED_DOWNLOADER
    }

    // the token will cancel task
    pub fn new_task(
        &self,
        task: DownloadTask,
        token: CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        let client = self.client.clone();
        let url = task.url.clone();
        let path = task.path.clone();
        let tracer = self.tracer.clone();
        tokio::spawn(async move {
            tracer.trace_task(task.clone()).await;
            let executer = async {
                let request = client.get(url.clone()).send().await;
                match request {
                    Ok(response) => {
                        let data = response.bytes().await;
                        match data {
                            Ok(contents) => match tokio::fs::write(path, contents).await {
                                Ok(_) => TaskStatus::OK,
                                Err(err) => TaskStatus::ERR(err.to_string()),
                            },
                            Err(err) => TaskStatus::ERR(err.to_string()),
                        }
                    }
                    Err(err) => TaskStatus::ERR(err.to_string()),
                }
            };

            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        tracer.stop_trace_task(task, TaskStatus::CANCEL).await;
                        break;
                    }
                    status = executer => {
                        tracer.stop_trace_task(task, status).await;
                        break;
                    }
                }
            }
        })
    }

    // the token will cancel all tasks
    pub fn new_tasks(
        &self,
        tasks: Vec<DownloadTask>,
        token: CancellationToken,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        tasks
            .into_iter()
            .map(|task| self.new_task(task, token.clone()))
            .collect()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DownloadTask {
    pub url: String,
    pub path: String,
    pub task_group_name: Option<String>,
    pub callback: DownloadTaskCallback,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum DownloadTaskCallback {
    DEFAULT,
    NATIVELIB(String, String, Vec<String>), // src, dest, exclude
    NONE,
}

impl DownloadTask {
    pub fn new(
        url: impl Into<String>,
        path: impl Into<String>,
        task_group_name: Option<String>,
    ) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            task_group_name: task_group_name.map(|v| v.into()),
            callback: DownloadTaskCallback::NONE,
        }
    }
    pub fn new_callback(
        url: impl Into<String>,
        path: impl Into<String>,
        task_group_name: Option<String>,
        callback: DownloadTaskCallback,
    ) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            task_group_name: task_group_name.map(|v| v.into()),
            callback,
        }
    }
}

#[tokio::test]
async fn test() {
    let downloader = ElementalDownloader::new();
    println!("start");
    let _ = downloader
        .new_task(
            DownloadTask::new(
                "http://launchermeta.mojang.com/mc/game/version_manifest.json",
                "version_manifest.json",
                None,
            ),
            CancellationToken::new(),
        )
        .await;
    println!("end");
}
