use tokio_util::sync::CancellationToken;

pub struct ElementalDownloader {
    client: reqwest::Client,
}

impl ElementalDownloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    // the token will cancel task
    pub fn new_task(
        &self,
        url: impl Into<String>,
        path: impl Into<String>,
        token: CancellationToken,
        callback: Option<fn(status: bool, url: String)>,
    ) {
        let client = self.client.clone();
        let url = url.into();
        let path = path.into();
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
                            func(val, url);
                        }
                        break;
                    }
                }
            }
        });
    }

    // the token will cancel all tasks
    pub fn new_tasks(
        &self,
        tasks: Vec<(impl Into<String>, impl Into<String>)>,
        token: CancellationToken,
        callback: Option<fn(status: bool, url: String)>,
    ) {
        for (url, path) in tasks {
            self.new_task(url, path, token.clone(), callback);
        }
    }
}
