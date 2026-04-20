use std::env::consts::{ARCH, OS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionJsonPlatform {
    os: String,
    arch: String,
}

impl VersionJsonPlatform {
    pub fn new(os: String, arch: String) -> Self {
        Self { os, arch }
    }

    pub fn current() -> Self {
        Self::new(OS.to_owned(), ARCH.to_owned())
    }

    pub fn os(&self) -> &str {
        &self.os
    }

    pub fn arch(&self) -> &str {
        &self.arch
    }
}
