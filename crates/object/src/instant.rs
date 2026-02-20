use async_trait::async_trait;
use std::{any::Any, sync::Arc};

#[async_trait]
pub trait InstantObject: Send + Sync {
    type Output: Any + Send + Sync;
    async fn create() -> Self::Output;
    async fn destroy(_this: Arc<Self::Output>) {
        // NO-OP
    }
}
