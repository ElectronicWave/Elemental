mod client;
mod endpoints;
mod profile_normalize;
mod profile_parse;
mod release;

pub use client::RiftSource;
pub use endpoints::{RiftEndpoints, RiftOrigin};
pub use release::RiftRelease;

fn with_trailing_slash(url: &str) -> String {
    if url.is_empty() || url.ends_with('/') {
        return url.to_owned();
    }

    format!("{url}/")
}
