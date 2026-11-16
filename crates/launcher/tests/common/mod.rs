#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn temp_root(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{prefix}-{}-{nonce}", std::process::id()))
}

pub fn remove(root: &Path) {
    fs::remove_dir_all(root).expect("remove test directory");
}
