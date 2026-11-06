# elemental-infra

Async download orchestration and JAR archive utilities for Rust applications.

`elemental-infra` is the download engine used by Elemental. It downloads files
through bounded concurrency, retries, rate limiting, optional SHA-1 validation,
and pluggable materialization. JAR reading and extraction helpers are included
for launcher-style workloads.

## Download model

| Type | Purpose |
| --- | --- |
| `DownloadTask` | One file: URL, destination, expected size, SHA-1, and request headers |
| `DownloadBatch` | A batch of tasks with an optional group |
| `DownloadSession` | One logical operation, such as installing one Minecraft version |
| `DownloadReport` | Final per-task outcomes plus byte and timing totals |
| `DownloadProgress` | Live task counts, byte counters, throughput, and ETA |
| `DownloadEvent` | Session and task events, serializable with the `serde` feature |
| `SessionSnapshot` | Current progress and task states for initial render or resync |
| `RetryPolicy` | Per-task attempts and exponential backoff |
| `FailurePolicy` | What a session does after a task exhausts its retries |

All downloader types are exported from `elemental_infra::downloader`. The
submodules that hold the implementation are private, so callers have one import
path instead of choosing between `core`, `task`, and `report`.

`ElementalDownloader` is a factory and owner of shared resources such as the
connection budget and bandwidth limiter. It does not represent one operation.
Each operation is a `DownloadSession`: open a session, submit tasks or batches
to it, then finish it to get a report. This keeps related downloads together
and lets the caller observe progress or inspect failures as a unit. Pass
`&DownloadSession` to code that performs downloads instead of passing the
downloader and creating hidden sessions.

Every task is validated before its batch is accepted. URLs must use `http` or
`https`, target paths must name a file and must not contain `..` components, and
SHA-1 checksums must be 40 hexadecimal characters. Validation covers the whole
batch first, so one malformed task cannot leave a half-submitted batch. This
keeps remote metadata from escaping the download root through a crafted library
path or cache key.

Groups are labels on batches. They do not change scheduling or task identity;
they only make reports easier to filter. Every task in a batch shares the
batch's group. A batch is submitted under one session lock, so concurrent
submitters cannot interleave the tasks of one group with another group.

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

Enable the `serde` feature when events, snapshots, progress, or reports need to
be serialized:

```toml
elemental-infra = { version = "0.2", features = ["serde"] }
serde_json = "1"
```

## Quick start

Download one file without validation:

```rust
use elemental_infra::downloader::{DownloadTask, ElementalDownloader};

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
use elemental_infra::downloader::{DownloadTask, ElementalDownloader};

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

Set headers on a session to apply them to every task, or on a task to override
the session value for that request:

```rust
use elemental_infra::downloader::DownloadTask;

let session = downloader
    .session("authenticated")
    .header("Authorization", format!("Bearer {token}"))?
    .open()
    .await?;

let task = DownloadTask::new(url, path)
    .header("Accept", "application/octet-stream")?;

session.submit(task).await?;
```

Task headers replace session headers with the same name; headers with different
names are merged. Use `.headers(HeaderMap)` to set a prebuilt map, which replaces
the current map. Header values are redacted from `Debug` output.

Batch work uses a session directly:

```rust
use elemental_infra::downloader::{
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
use elemental_infra::downloader::{
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
immediately. A retry releases the shared connection permit before waiting for
its backoff, so one sleeping task cannot occupy a download slot that another
queued task could use.

```rust
use std::time::Duration;

use elemental_infra::downloader::{
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
use elemental_infra::downloader::{
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

## Live events and progress

`session.subscribe()` returns a session-scoped event stream, while
`downloader.subscribe()` receives events for every session. Events are
emitted for session start/close, task queue/start/finish, retries, aggregate
progress, and per-task progress. With the `serde` feature enabled, they are
also serializable. Progress events are throttled to 10 Hz; lifecycle events
are emitted once.

`SessionOpened` is emitted before `open()` returns, so it is only observable
through `downloader.subscribe()`. A session-scoped subscriber starts from the
current state; use `session.snapshot()` for its initial render.

```rust
use elemental_infra::downloader::{
    DownloadBatch, DownloadEvent, ElementalDownloader, EventStreamError,
};

async fn download_with_events(
    downloader: &ElementalDownloader,
    batch: DownloadBatch,
) -> anyhow::Result<()> {
    let session = downloader.session("assets").open().await?;
    let monitor_session = session.clone();
    let mut events = session.subscribe();
    session.submit(batch).await?;

    let monitor = tokio::spawn(async move {
        loop {
            match events.recv().await {
                Ok(DownloadEvent::SessionClosed { .. }) => break,
                Ok(event) => println!("{}", serde_json::to_string(&event)?),
                Err(EventStreamError::Lagged(_)) => {
                    let snapshot = monitor_session.snapshot().await?;
                    println!("{}", serde_json::to_string(&snapshot)?);
                }
                Err(EventStreamError::Closed) => break,
            }
        }

        Ok::<(), anyhow::Error>(())
    });

    let report = session.finish().await?;
    report.success()?;
    monitor.await?;
    Ok(())
}
```

Use `session.snapshot()` to initialize the frontend or recover after a
`Lagged` event. The snapshot contains the current aggregate progress and every
task state. `session.progress()` and `progress_receiver()` remain available for
consumers that only need the aggregate `DownloadProgress`.

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
Dropping the last clone without calling `finish()` or `cancel()` also cancels
and removes the session, so a forgotten session cannot keep the downloader
alive. Prefer the explicit methods when you need the final report.

## Configuration

| Setting | Default |
| --- | --- |
| `max_concurrent_downloads` | 8 |
| `default_session_parallelism` | 8 |
| `session_queue_capacity` | 256 |
| `connect_timeout` | 10 seconds |
| `read_timeout` | 30 seconds |
| `rate_limit` | Unlimited |
| session retry | 3 attempts, 500 ms to 8 s backoff |

Rate limiting uses a token bucket shared by the downloader and an optional
second bucket per session. Large chunks are charged in full instead of being
clamped to one second of budget, so the configured rate is an actual ceiling.

Override the defaults through `ElementalDownloaderBuilder::config()`:

```rust
use std::time::Duration;

use elemental_infra::downloader::{
    ByteRate, DownloadRateLimit, ElementalDownloader, ElementalDownloaderConfig,
};

fn build_downloader() -> anyhow::Result<std::sync::Arc<ElementalDownloader>> {
    let config = ElementalDownloaderConfig {
        max_concurrent_downloads: 16,
        default_session_parallelism: 4,
        session_queue_capacity: 256,
        connect_timeout: Duration::from_secs(15),
        read_timeout: Duration::from_secs(30),
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

use elemental_infra::downloader::{
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

`UncachedMaterializer` is the default. Each download is staged next to its
destination and committed only after validation succeeds.

`HardlinkCacheMaterializer` stores verified content in a SHA-1-addressed cache
and hardlinks it into the destination. Tasks without a SHA-1 use the non-cached
path.

```rust
use elemental_infra::downloader::ElementalDownloader;

fn main() -> anyhow::Result<()> {
    let _downloader = ElementalDownloader::builder()
        .with_hardlink_cache(".cache/elemental")
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
