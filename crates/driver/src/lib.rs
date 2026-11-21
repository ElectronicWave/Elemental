//! Driver implementations for Minecraft versions and mod loaders.
//!
//! A driver inspects and prepares local game instances, resolves launch
//! metadata, and provides catalogs for available game and loader versions.
//! Drivers never execute the game directly; they produce data consumed by the
//! launcher layer.

#[cfg(not(any(
    feature = "tls-aws-lc",
    feature = "tls-rustls-no-provider",
    feature = "tls-native",
    feature = "tls-native-vendored"
)))]
compile_error!(
    "`elemental-driver` requires a TLS backend. Enable one of `tls-aws-lc` \
     (default), `tls-rustls-no-provider`, `tls-native`, or \
     `tls-native-vendored`. When using `default-features = false`, add a TLS \
     backend feature explicitly."
);

pub mod catalog;
pub mod descriptors;
pub mod driver;
pub mod drivers;
pub mod error;
pub mod families;
mod http;
pub mod inspect;
pub mod launch;
pub mod loader_version;
mod maven;
pub mod url;
