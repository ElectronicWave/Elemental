use std::fmt::Debug;

use anyhow::Result;

pub trait VersionJsonRemoteResolver: Clone + Debug + Send + Sync + 'static {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String>;
    fn object_url(&self, hash: &str) -> Result<String>;
}
