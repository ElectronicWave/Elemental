use std::sync::Arc;
use tokio::sync::{Mutex as AsyncMutex, broadcast, mpsc, watch};

use super::SessionHandler;
use crate::downloader::core::ElementalDownloader;
use crate::downloader::event::DownloadEvent;
use crate::downloader::limits::BandwidthLimiter;
use crate::downloader::report::DownloadProgress;
use crate::downloader::session::DownloadSessionOptions;
use crate::downloader::task::SessionId;
use crate::downloader::worker::run_session_worker;
use crate::error::Result;

impl SessionHandler {
    pub(crate) fn spawn(
        session_id: SessionId,
        name: Option<String>,
        options: DownloadSessionOptions,
        queue_capacity: usize,
        default_parallelism: usize,
        downloader: Arc<ElementalDownloader>,
        events: broadcast::Sender<DownloadEvent>,
    ) -> Result<(Arc<Self>, watch::Receiver<DownloadProgress>)> {
        let parallelism = options.effective_parallelism(default_parallelism);
        let bandwidth_limiter = Arc::new(BandwidthLimiter::new(options.effective_rate_limit())?);
        let headers = options.headers.clone();
        let (sender, receiver) = mpsc::channel(queue_capacity);
        let receiver = Arc::new(AsyncMutex::new(receiver));
        let (handler, progress) = Self::new(
            session_id,
            name,
            options,
            sender,
            headers,
            bandwidth_limiter,
            events,
        );
        let handler = Arc::new(handler);

        for _ in 0..parallelism {
            let worker_handler = Arc::clone(&handler);
            let worker_receiver = Arc::clone(&receiver);
            let worker_downloader = Arc::clone(&downloader);
            handler.spawn_worker(async move {
                run_session_worker(worker_downloader, worker_handler, worker_receiver).await;
            });
        }

        Ok((handler, progress))
    }
}
