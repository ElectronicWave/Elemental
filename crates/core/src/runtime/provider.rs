use async_trait::async_trait;
use std::{path::PathBuf, sync::Arc};

#[async_trait]
pub trait RuntimeProvider: Send + Sync {
    async fn list(&self) -> Vec<PathBuf>;

    #[inline(always)]
    fn name(&self) -> &'static str {
        "Provider"
    }

    fn arc_default() -> Arc<Self>
    where
        Self: Sized + Default,
    {
        Arc::new(Self::default())
    }
}

pub type RuntimeProviderList = Vec<Arc<dyn RuntimeProvider>>;
