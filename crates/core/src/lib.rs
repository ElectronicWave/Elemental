/// Emits a compile error when the invoking crate has not enabled any TLS backend feature.
#[macro_export]
macro_rules! require_tls_backend {
    () => {
        #[cfg(not(any(
            feature = "tls-aws-lc",
            feature = "tls-rustls-no-provider",
            feature = "tls-native",
            feature = "tls-native-vendored"
        )))]
        compile_error!(concat!(
            "`",
            env!("CARGO_PKG_NAME"),
            "` requires a TLS backend. Enable one of `tls-aws-lc` (default), \
             `tls-rustls-no-provider`, `tls-native`, or `tls-native-vendored`. \
             When using `default-features = false`, add a TLS backend feature explicitly."
        ));
    };
}

#[cfg(any(
    feature = "http",
    feature = "microsoft-auth",
    feature = "yggdrasil-auth"
))]
require_tls_backend!();

pub mod auth;
#[cfg(feature = "http")]
pub mod http;
pub mod launcher;
pub mod minecraft;
pub mod runtime;
pub mod storage;
#[cfg(feature = "http")]
pub mod url;
pub mod version;
