use std::env::consts::{ARCH, OS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MojangPlatform {
    os: String,
    arch: String,
}

impl MojangPlatform {
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
