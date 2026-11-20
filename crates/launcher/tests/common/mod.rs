#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn temp_root(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{prefix}-{}-{nonce}-{counter}", std::process::id()))
}

pub fn remove(root: &Path) {
    fs::remove_dir_all(root).expect("remove test directory");
}

pub fn texture_hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
