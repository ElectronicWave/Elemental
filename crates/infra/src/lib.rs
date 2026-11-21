#[cfg(not(any(
    feature = "tls-aws-lc",
    feature = "tls-rustls-no-provider",
    feature = "tls-native",
    feature = "tls-native-vendored"
)))]
compile_error!(
    "`elemental-infra` requires a TLS backend. Enable one of `tls-aws-lc` \
     (default), `tls-rustls-no-provider`, `tls-native`, or \
     `tls-native-vendored`. When using `default-features = false`, add a TLS \
     backend feature explicitly."
);

pub mod downloader;
pub mod error;
pub mod jar;
pub mod path;

pub use path::SafeRelativePath;
