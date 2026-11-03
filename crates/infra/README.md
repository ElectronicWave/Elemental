# elemental-infra

Async download orchestration and JAR archive utilities for Rust applications.

`elemental-infra` provides the download engine used by Elemental and a small
set of archive helpers that are useful on their own. Downloads are grouped into
sessions and plans, with bounded concurrency, retries, rate limiting, optional
SHA-1 validation, progress snapshots, and pluggable materialization.

## Features

- Global and per-session concurrency limits.
- Named or unnamed download sessions and reusable download plans.
- Connect timeouts, retry policy, and per-task size or SHA-1 validation.
- Globally enforced or per-session bandwidth limits.
- Progress, failure, and cancellation snapshots.
- Uncached materialization by default, with an optional hardlink cache.
- Blocking ZIP/JAR reading and extraction helpers with safe path handling.

## Installation

```toml
[dependencies]
anyhow = "1"
elemental-infra = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

Download a single file with a one-shot plan:

```rust
use std::path::PathBuf;

use elemental_infra::downloader::core::{
    DownloadExecutionPolicy, DownloadPlan, DownloadTask, ElementalDownloader,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let downloader = ElementalDownloader::builder().build()?;

    let plan = DownloadPlan::named(
        "manifest",
        DownloadExecutionPolicy::ServiceDefault,
        vec![DownloadTask::new(
            "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
            PathBuf::from(".minecraft/version_manifest_v2.json"),
            None,
            None,
        )],
    )?;

    downloader.run_plan(plan).await?;
    Ok(())
}
```

`run_plan()` closes the internal session, validates failures and cancellations,
and removes the session afterward. Use `execute_plans()` for a sequence of
plans or a `DownloadPlanner` implementation.

## Download Model

| Type | Purpose |
| --- | --- |
| `DownloadTask` | Source URL, destination path, expected size, and optional SHA-1 |
| `DownloadSessionRequest` | Session name and execution policy |
| `DownloadPlan` | A session request plus its tasks |
| `DownloadSession` | Handle for adding tasks and observing progress |
| `DownloadSessionSnapshot` | Counts, throughput, failures, and cancellation state |
| `DownloadExecutionPolicy` | Service defaults or explicit parallelism and rate limit |

### Sessions

Sessions are useful when work arrives incrementally or the host application
wants progress updates:

```rust
use std::path::PathBuf;

use elemental_infra::downloader::core::{
    DownloadExecutionPolicy, DownloadSessionSnapshot, DownloadTask, ElementalDownloader,
};

async fn download_asset(
    downloader: &ElementalDownloader,
    url: String,
    path: PathBuf,
    size: u64,
    sha1: String,
) -> anyhow::Result<DownloadSessionSnapshot> {
    let session = downloader
        .create_named_session("assets", DownloadExecutionPolicy::ServiceDefault)
        .await?;

    session
        .add_task(DownloadTask::new(url, path, Some(size), Some(sha1)))
        .await?;

    session.finish_snapshot().await
}
```

> [!IMPORTANT]
> `finish()` closes the session and waits for its workers, but it does not
> report task failures itself. Use `finish_snapshot()` when failures and
> cancellations must be inspected, or prefer `run_plan()` for one-shot work.

## Default Configuration

| Setting | Default |
| --- | --- |
| `max_connections` | 8 |
| `session_parallelism` | 8 |
| `session_queue_capacity` | 256 |
| `connect_timeout` | 10 seconds |
| `retry_times` | 3 |
| `rate_limit` | Unlimited |

Override the configuration through `ElementalDownloaderBuilder::config()`, or
set a session-specific policy with
`DownloadExecutionPolicy::Custom { parallelism, rate_limit }`.

## Materialization

`NoCachedMaterializer` is the default. It stages each download next to its
destination and commits it when validation succeeds.

`HardlinkCachedMaterializer` stores verified content in a SHA-1-addressed cache
and hardlinks it into the destination. Tasks without a `sha1` are not
content-addressable and use the non-cached path.

```rust
use elemental_infra::downloader::core::ElementalDownloader;

fn main() -> anyhow::Result<()> {
    let _downloader = ElementalDownloader::builder()
        .hardlink_cached_materializer(".cache/elemental")
        .build()?;
    Ok(())
}
```

## JAR Utilities

`JarFile` reads from disk, while `JarBytes` reads directly from a byte slice.
Both can return a named entry as bytes or UTF-8 text.

| API | Behavior |
| --- | --- |
| `JarFile::by_name_bytes` | Read one entry from a file |
| `JarFile::by_name_string` | Read one UTF-8 entry from a file |
| `JarFile::extract_blocking` | Extract a JAR while skipping `META-INF` |
| `JarFile::extract_prefixed_blocking` | Extract only a prefix and strip that prefix |
| `JarBytes::by_name_*` | Read entries without extracting the archive |

Archive extraction uses each entry's enclosed path and skips entries that
would escape the destination directory.

```rust
use elemental_infra::jar::JarFile;

fn main() -> anyhow::Result<()> {
    let manifest =
        JarFile::new("example.jar").by_name_string("fabric.mod.json")?;
    println!("{manifest}");
    Ok(())
}
```

The archive APIs are blocking and should run outside async worker threads when
files are large, for example with `tokio::task::spawn_blocking`.

## Related Crates

- [elemental-driver](https://github.com/ElectronicWave/Elemental/blob/main/crates/driver/README.md)
  uses the downloader to install Minecraft distributions.
- [elemental-launcher](https://github.com/ElectronicWave/Elemental/blob/main/crates/launcher/README.md)
  shares one downloader across instance preparation.
- [elemental-kit](https://github.com/ElectronicWave/Elemental/blob/main/README.md#the-elemental-kit-facade)
  re-exports this crate behind the `infra` feature.
