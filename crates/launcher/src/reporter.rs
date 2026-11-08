use elemental_infra::downloader::DownloadProgress;

pub trait ProgressReporter: Send + Sync {
    fn report(&self, progress: &DownloadProgress);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopReporter;

impl ProgressReporter for NoopReporter {
    fn report(&self, _progress: &DownloadProgress) {}
}

pub struct CallbackReporter<F> {
    callback: F,
}

impl<F> CallbackReporter<F> {
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> ProgressReporter for CallbackReporter<F>
where
    F: Fn(&DownloadProgress) + Send + Sync,
{
    fn report(&self, progress: &DownloadProgress) {
        (self.callback)(progress);
    }
}
