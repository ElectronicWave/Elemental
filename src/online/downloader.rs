use std::sync::LazyLock;
use tokio_util::sync::CancellationToken;

pub struct ElementalDownloader {
    client: reqwest::Client,
}

static SHARED_DOWNLOADER: LazyLock<ElementalDownloader> = LazyLock::new(ElementalDownloader::new);

impl ElementalDownloader {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
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
        callback: Option<fn(status: bool, task: DownloadTask)>,
    ) -> tokio::task::JoinHandle<()> {
        let client = self.client.clone();
        let url = task.url.clone();
        let path = task.path.clone();
        tokio::spawn(async move {
            let executer = async {
                let request = client.get(url.clone()).send().await;
                if let Ok(response) = request {
                    let data = response.bytes().await;
                    if let Ok(contents) = data {
                        if tokio::fs::write(path, contents).await.is_ok() {
                            return true;
                        }
                    }
                }
                false
            };

            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        // cancel task
                        break;
                    }
                    val = executer => {
                        if let Some(func) = callback {
                            func(val, task);
                        }
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
        callback: Option<fn(status: bool, task: DownloadTask)>,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        tasks
            .into_iter()
            .map(|task| self.new_task(task, token.clone(), callback))
            .collect()
    }
}

#[derive(Debug)]
pub struct DownloadTask {
    pub url: String,
    pub path: String,
    pub size: Option<usize>,
}

impl DownloadTask {
    pub fn new(url: impl Into<String>, path: impl Into<String>, size: Option<usize>) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            size,
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
            None,
        )
        .await;
    println!("end");
}
