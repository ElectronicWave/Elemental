#[cfg(all(
    any(feature = "microsoft-auth", feature = "yggdrasil-auth"),
    not(any(
        feature = "tls-aws-lc",
        feature = "tls-rustls-no-provider",
        feature = "tls-native",
        feature = "tls-native-vendored"
    ))
))]
compile_error!(
    "HTTP authentication is enabled without a TLS backend. Enable one of \
     `tls-aws-lc` (default), `tls-rustls-no-provider`, `tls-native`, or \
     `tls-native-vendored`. When using `default-features = false`, add a TLS \
     backend feature explicitly."
);

pub mod auth;
pub mod launcher;
pub mod minecraft;
pub mod runtime;
pub mod storage;
pub mod version;
