use std::sync::Arc;

use tokio::task_local;

use crate::pool::ObjectPool;

#[derive(Clone)]
pub struct ObjectContext {
    inner: Arc<ObjectPool>,
}

task_local! {
    pub static CONTEXT: Arc<ObjectPool>;
}

impl ObjectContext {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ObjectPool::new()),
        }
    }

    pub async fn run<Fut>(self, f: Fut) -> Fut::Output
    where
        Fut: Future,
    {
        CONTEXT.scope(self.inner.clone(), f).await
    }

    pub async fn shutdown(self) {
        CONTEXT
            .scope(self.inner.clone(), async {
                let pool = CONTEXT.with(|pool| pool.clone());
                pool.shutdown().await;
            })
            .await;
    }
}
