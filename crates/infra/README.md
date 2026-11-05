# elemental-infra

Async download orchestration and JAR archive utilities for Rust applications.

`elemental-infra` is the download engine used by Elemental. It downloads files
through bounded concurrency, retries, rate limiting, optional SHA-1 validation,
and pluggable materialization. JAR reading and extraction helpers are included
for launcher-style workloads.

## Download model

| Type | Purpose |
| --- | --- |
| `DownloadTask` | One file: URL, destination, expected size, and SHA-1 |
| `DownloadBatch` | A batch of tasks with an optional group |
| `DownloadSession` | One logical operation, such as installing one Minecraft version |
| `DownloadReport` | Final per-task outcomes plus byte and timing totals |
| `DownloadProgress` | Live task counts, byte counters, throughput, and ETA |
| `RetryPolicy` | Per-task attempts and exponential backoff |
| `FailurePolicy` | What a session does after a task exhausts its retries |

`ElementalDownloader` is a factory and owner of shared resources such as the
connection budget and bandwidth limiter. It does not represent one operation.
Each operation is a `DownloadSession`: open a session, submit tasks or batches
to it, then finish it to get a report. This keeps related downloads together
and lets the caller observe progress or inspect failures as a unit. Pass
`&DownloadSession` to code that performs downloads instead of passing the
downloader and creating hidden sessions.

Groups are labels on batches. They do not change scheduling or task identity;
they only make reports easier to filter. Every task in a batch shares the
batch's group.

Minecraft preparation uses one session per version with four groups:

| Group | Contents |
| --- | --- |
| `installer` | Installer archive |
| `version` | Client jar, libraries, and logging configuration |
| `assets` | Asset objects |
| `profile-libraries` | Extra libraries from the installer profile |

## Installation

```toml
[dependencies]
anyhow = "1"
elemental-infra = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick start

Download one file without validation:

```rust
use elemental_infra::downloader::core::{DownloadTask, ElementalDownloader};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let downloader = ElementalDownloader::builder().build()?;

    let report = downloader
        .download(DownloadTask::new(
            "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
            ".minecraft/version_manifest_v2.json",
        ))
        .await?;

    println!(
        "downloaded {} bytes in {:?}",
        report.downloaded_bytes, report.elapsed
    );
    Ok(())
}
```

`download()` returns an error if the task fails. Add validation when the source
publishes a size or SHA-1:

```rust
use elemental_infra::downloader::core::{DownloadTask, ElementalDownloader};

async fn download_library(
    downloader: &ElementalDownloader,
    url: String,
    path: String,
    size: u64,
    sha1: String,
) -> anyhow::Result<()> {
    let task = DownloadTask::new(url, path).size(size).sha1(sha1);

    downloader.download(task).await?;
    Ok(())
}
```

Batch work uses a session directly:

```rust
use elemental_infra::downloader::core::{
    DownloadTask, ElementalDownloader,
};

async fn download_batch(
    downloader: &ElementalDownloader,
    tasks: Vec<DownloadTask>,
) -> anyhow::Result<()> {
    let session = downloader.session("batch").open().await?;
    session.submit(tasks).await?;
    let report = session.finish().await?;

    for outcome in &report.outcomes {
        println!("{}: {:?}", outcome.task_id, outcome.status);
    }

    report.success()?;
    Ok(())
}
```

## Batches and groups

Build batches with `DownloadBatch::new()` and label them with
`DownloadBatch::group()`:

```rust
use elemental_infra::downloader::core::{
    DownloadBatch, DownloadTask, ElementalDownloader,
};

async fn install_version(
    downloader: &ElementalDownloader,
    version: &str,
    version_tasks: Vec<DownloadTask>,
    asset_tasks: Vec<DownloadTask>,
) -> anyhow::Result<()> {
    let session = downloader.session(version).open().await?;

    session
        .submit(DownloadBatch::new(version_tasks).group("version"))
        .await?;
    session
        .submit(DownloadBatch::new(asset_tasks).group("assets"))
        .await?;

    let report = session.finish().await?;

    for failure in report.failures() {
        eprintln!("{}", failure.summary());
    }
    report.success()?;
    Ok(())
}
```

`DownloadBatch::group("assets")` applies `"assets"` to every task in that
batch. Batches without a group still download normally, but their outcomes
have no group label.

## Retry and failure strategy

Retries belong to the task that failed. Each task gets its own attempt budget,
and only transient failures are retried: connection errors, interrupted
streams, HTTP 408, HTTP 429, HTTP 5xx, and verification mismatches. HTTP 4xx
responses other than 408 and 429, prepare errors, and commit errors fail
immediately.

```rust
use std::time::Duration;

use elemental_infra::downloader::core::{
    DownloadTask, ElementalDownloader, FailurePolicy, RetryPolicy,
};

async fn install_assets(
    downloader: &ElementalDownloader,
    tasks: Vec<DownloadTask>,
) -> anyhow::Result<()> {
    let session = downloader
        .session("assets")
        .retry(RetryPolicy {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(8),
        })
        .failure_policy(FailurePolicy::Continue)
        .open()
        .await?;

    session.submit(tasks).await?;
    let report = session.finish().await?;
    report.success()?;
    Ok(())
}
```

The failure policy runs only after a task has used all of its attempts:

| Policy | Behavior |
| --- | --- |
| `Continue` | Keep running queued and future tasks, then aggregate failures |
| `FinishInFlight` | Let started tasks finish, cancel queued tasks, and reject later submissions |
| `CancelAll` | Cancel running and queued tasks immediately, and reject later submissions |

`FinishInFlight` does not hide the original failure. The report contains the
failed task, and later `submit()` calls return
`Error::SessionNotAcceptingTasks`. Use `report.retryable_failures()` when you
want to build an explicit reconciliation step outside the session; the
downloader does not retry the whole session automatically.

## Incremental sessions

Use a session directly when work arrives incrementally and installed artifacts
are needed before the next step. `wait_idle()` waits for queued work without
closing the session, so more tasks can be added afterward:

```rust
use elemental_infra::downloader::core::{
    DownloadBatch, DownloadTask, ElementalDownloader,
};

async fn prepare_in_steps(
    downloader: &ElementalDownloader,
    installer_tasks: Vec<DownloadTask>,
    version_tasks: Vec<DownloadTask>,
) -> anyhow::Result<()> {
    let session = downloader
        .session("forge 1.20.1 47.2.0")
        .open()
        .await?;

    session
        .submit(DownloadBatch::new(installer_tasks).group("installer"))
        .await?;
    session.wait_idle().await?;

    session
        .submit(DownloadBatch::new(version_tasks).group("version"))
        .await?;
    session.wait_idle().await?;

    let report = session.finish().await?;
    report.success()?;
    Ok(())
}
```

`finish()` closes the session, waits for its workers, returns a
`DownloadReport`, and removes the session from the downloader.

Use `session.success()` when an operation needs a checkpoint between
steps. It waits for the session to become idle and returns an error if any task
has failed or been cancelled, but it leaves the session open so more work can
be submitted afterward. Call `finish()` once at the end of the operation to
close the session and collect the final report.

## Live progress

`progress_receiver()` returns a `tokio::sync::watch::Receiver`. It emits
updates whenever a task is queued, finishes, fails, or receives bytes:

```rust
use elemental_infra::downloader::core::{
    DownloadBatch, ElementalDownloader,
};

async fn download_with_progress(
    downloader: &ElementalDownloader,
    batch: DownloadBatch,
) -> anyhow::Result<()> {
    let session = downloader.session("assets").open().await?;
    session.submit(batch).await?;

    let mut progress = session.progress_receiver();
    let monitor = tokio::spawn(async move {
        while progress.changed().await.is_ok() {
            let current = progress.borrow().clone();
            println!(
                "{}/{} tasks, {} bytes, {:?}",
                current.tasks_completed,
                current.tasks_total,
                current.bytes_downloaded,
                current.eta
            );
        }
    });

    let report = session.finish().await?;
    report.success()?;
    monitor.await?;
    Ok(())
}
```

Use `session.progress()` for a synchronous snapshot of the latest value.

## Failures and cancellation

- `download()` and `report.success()` return an error when any task
  failed or was cancelled.
- Session `finish()` and `cancel()` return the report without failing on task
  errors. Call `success()` when that is not what you want.
- `report.failures()` yields structured `DownloadFailure` values, including
  phase, HTTP status, group, path, and whether the failure was retryable.
- `report.outcomes_in_group("assets")` filters results by group.
- `report.retryable_failures()` filters failures that can be reconciled
  explicitly.
- `session.cancel().await?` cancels the session token, waits for workers, and
  returns a report. Queued tasks appear as cancelled.

`DownloadSession` is cloneable; cancelling any clone cancels the whole session.

## Configuration

| Setting | Default |
| --- | --- |
| `max_connections` | 8 |
| `session_parallelism` | 8 |
| `session_queue_capacity` | 256 |
| `connect_timeout` | 10 seconds |
| `rate_limit` | Unlimited |
| session retry | 3 attempts, 500 ms to 8 s backoff |

Override the defaults through `ElementalDownloaderBuilder::config()`:

```rust
use std::time::Duration;

use elemental_infra::downloader::core::{
    ByteRate, DownloadRateLimit, ElementalDownloader, ElementalDownloaderConfig,
};

fn build_downloader() -> anyhow::Result<std::sync::Arc<ElementalDownloader>> {
    let config = ElementalDownloaderConfig {
        max_connections: 16,
        session_parallelism: 4,
        session_queue_capacity: 256,
        connect_timeout: Duration::from_secs(15),
        rate_limit: DownloadRateLimit::Limited(ByteRate::mib_per_sec(
            std::num::NonZeroUsize::new(20).expect("non-zero rate"),
        )),
    };

    Ok(ElementalDownloader::builder().config(config).build()?)
}
```

Override parallelism or the session rate limit on the session builder:

```rust
use std::num::NonZeroUsize;

use elemental_infra::downloader::core::{
    ByteRate, DownloadRateLimit, ElementalDownloader, FailurePolicy, RetryPolicy,
};

let session = downloader
    .session("assets")
    .parallelism(4)
    .retry(RetryPolicy::none())
    .failure_policy(FailurePolicy::CancelAll)
    .rate_limit(DownloadRateLimit::Limited(ByteRate::kib_per_sec(
        NonZeroUsize::new(512).expect("non-zero rate"),
    )))
    .open()
    .await?;
```

## Materialization

`NoCachedMaterializer` is the default. Each download is staged next to its
destination and committed only after validation succeeds.

`HardlinkCachedMaterializer` stores verified content in a SHA-1-addressed cache
and hardlinks it into the destination. Tasks without a SHA-1 use the non-cached
path.

```rust
use elemental_infra::downloader::core::ElementalDownloader;

fn main() -> anyhow::Result<()> {
    let _downloader = ElementalDownloader::builder()
        .hardlink_cached_materializer(".cache/elemental")
        .build()?;
    Ok(())
}
```

## JAR utilities

`JarFile` reads from disk, while `JarBytes` reads directly from a byte slice.
Both can return a named entry as bytes or UTF-8 text.

| API | Behavior |
| --- | --- |
| `JarFile::by_name_bytes` | Read one entry from a file |
| `JarFile::by_name_string` | Read one UTF-8 entry from a file |
| `JarFile::extract_blocking` | Extract a JAR while skipping `META-INF` |
| `JarFile::extract_prefixed_blocking` | Extract only a prefix and strip that prefix |
| `JarBytes::by_name_*` | Read entries without extracting the archive |

Archive extraction uses each entry's enclosed path and skips entries that would
escape the destination directory.

```rust
use elemental_infra::jar::JarFile;

fn main() -> anyhow::Result<()> {
    let manifest = JarFile::new("example.jar").by_name_string("fabric.mod.json")?;
    println!("{manifest}");
    Ok(())
}
```

The archive APIs are blocking. Run them outside async worker threads when files
are large, for example with `tokio::task::spawn_blocking`.

## Related crates

- [elemental-driver](https://github.com/ElectronicWave/Elemental/blob/main/crates/driver/README.md)
  uses the downloader to install Minecraft distributions.
- [elemental-launcher](https://github.com/ElectronicWave/Elemental/blob/main/crates/launcher/README.md)
  shares one downloader across instance preparation.
- [elemental-kit](https://github.com/ElectronicWave/Elemental/blob/main/README.md#the-elemental-kit-facade)
  re-exports this crate behind the `infra` feature.
