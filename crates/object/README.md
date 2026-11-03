# elemental-object

Type-indexed async object pool for long-lived Rust services.

`elemental-object` keeps one value per concrete type in a process-wide pool.
Use it for services, runtimes, and hot-reloadable state that would otherwise be
threaded through every function. Values are stored as `Arc<T>` and accessed
through a concurrent hash map. An optional task-local `ObjectContext` keeps
request- or tenant-scoped values separate from the global pool.

## Features

- One value per concrete type, keyed by `TypeId`.
- Concurrent access through `scc::HashMap` and shared `Arc<T>` ownership.
- Async retrieval and provisioning.
- Optional wait-for-initialization with the default `notify` feature.
- Async shutdown hooks when values are replaced or removed.
- Global and task-local object pools.
- `InstantObject` support for values with async creation and destruction.

## Installation

```toml
[dependencies]
anyhow = "1"
async-trait = "0.1"
elemental-object = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The default `notify` feature enables `acquire()` and waiter notifications.

## Quick Start

```rust
use anyhow::Result;
use elemental_object::{provide_with_shutdown, require, shutdown};

struct AppConfig {
    name: String,
    workers: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    provide_with_shutdown(
        AppConfig {
            name: "elemental".to_owned(),
            workers: 4,
        },
        |_config| async {
            println!("AppConfig shut down");
        },
    )
    .await;

    let config = require::<AppConfig>().await?;
    println!("{} with {} workers", config.name, config.workers);

    shutdown().await;
    Ok(())
}
```

`require()` returns an error immediately when the type is not present.
`provide()` inserts or replaces a value and runs the previous shutdown hook, if
one was registered.

## Core Concepts

### Global pool

The plain functions operate on a process-wide pool:

- `provide`, `provide_with_shutdown`
- `require`
- `acquire`
- `fulfill`, `fulfill_with_shutdown`
- `drop_value`, `drop_entry`
- `shutdown`

Stored values must be `Any + Send + Sync`. Retrieval returns `Arc<T>`, so
callers can share the value without copying it.

### Lifecycle and replacement

Providing a value for a type that already exists replaces the value and invokes
the old shutdown hook. This is the intended hot-reload path.

`drop_value::<T>()` removes the value but keeps the entry. `drop_entry::<T>()`
removes the whole entry. `shutdown()` runs the shutdown hook for every value in
the global pool.

### Waiting for initialization

`acquire::<T>()` waits until a value is provided, then returns it. If the value
is removed while a caller is waiting, `acquire()` returns an error instead of
blocking forever.

> [!IMPORTANT]
> `fulfill()` only fills a slot that already exists, normally created by a
> waiter in `acquire()`, and is idempotent. Use `provide()` when the producer
> may run before any consumer starts waiting.

### Task-local contexts

`ObjectContext` creates a task-local pool that is independent from the global
one. `ObjectContext::run()` scopes a future to that pool.

`require()` checks the active context before the global pool. The context-aware
providers target the active context when one exists and fall back to the global
pool otherwise. Plain `provide()` and `fulfill()` always target the global
pool. `acquire()` waits in the active context when one exists and does not
observe the global pool from that task. `drop_context_value()` and
`drop_context_entry()` are no-ops when no context is active.

## Common Patterns

### Wait for initialization

```rust
use anyhow::Result;
use elemental_object::{acquire, provide};

#[derive(Debug)]
struct Config {
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let waiter = tokio::spawn(async { acquire::<Config>().await });

    provide(Config { port: 25565 }).await;
    let config = waiter.await??;

    println!("port: {}", config.port);
    Ok(())
}
```

### Request-scoped values

```rust
use anyhow::Result;
use elemental_object::{ObjectContext, acquire, provide_context};

#[derive(Debug)]
struct Tenant {
    id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let context = ObjectContext::new();

    let producer = context.clone().run(async {
        provide_context(Tenant {
            id: "acme".to_owned(),
        })
        .await;
    });

    let consumer = context.clone().run(async {
        let tenant = acquire::<Tenant>().await?;
        println!("tenant: {}", tenant.id);
        Ok::<_, anyhow::Error>(())
    });

    let ((), result) = tokio::join!(producer, consumer);
    result?;
    context.shutdown().await;
    Ok(())
}
```

### InstantObject

`InstantObject` separates async construction and destruction from the value
that is stored in the pool:

```rust
use std::sync::Arc;

use async_trait::async_trait;
use elemental_object::{instant::InstantObject, provide_instant, require};

struct Database;

struct Connection;

#[async_trait]
impl InstantObject for Database {
    type Output = Connection;

    async fn create() -> Self::Output {
        Connection
    }

    async fn destroy(_connection: Arc<Self::Output>) {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    provide_instant::<Database>().await;
    let _connection = require::<Connection>().await?;
    Ok(())
}
```

## API Overview

| API | Pool | Behavior |
| --- | --- | --- |
| `require::<T>()` | Active context, then global | Retrieve a value or return an error |
| `acquire::<T>()` | Active context when present; otherwise global | Wait for a value, then retrieve it |
| `provide`, `provide_with_shutdown` | Global | Insert or replace a value |
| `provide_context`, `provide_context_with_shutdown` | Active context, then global | Insert or replace in a context pool |
| `fulfill`, `fulfill_with_shutdown` | Global | Fill an existing slot without replacing a value |
| `fulfill_context`, `fulfill_with_context_shutdown` | Active context, then global | Fill an existing contextual slot |
| `provide_instant`, `provide_instant_context` | Global or context | Create and register an `InstantObject` |
| `drop_value`, `drop_entry` | Global | Remove a value or entry |
| `drop_context_value`, `drop_context_entry` | Active context | Remove from a context pool |
| `shutdown`, `shutdown_local` | Global or context | Run all registered shutdown hooks |

`read`-style APIs do not mutate stored values. Replace or remove values through
the provided lifecycle functions so shutdown hooks stay consistent.

## Feature Flags

`notify` is enabled by default and adds the Tokio notification dependency plus
the `acquire()` API. To build without waiter support:

```toml
[dependencies]
elemental-object = { version = "0.2", default-features = false }
```

## Error Handling

`require()` returns `anyhow::Result<Arc<T>>`. A missing value produces an error
that includes the requested type name, for example:

```text
Cannot get object `my_app::config::Config` from pool
```

`acquire()` returns the same kind of error when the value disappears while it
is waiting.

## Relationship to Elemental

This crate is part of the [Elemental](https://github.com/ElectronicWave/Elemental)
workspace and is also available through the `elemental-kit` facade.

For implementation details and concurrency notes, see
[DESIGN.md](https://github.com/ElectronicWave/Elemental/blob/main/crates/object/DESIGN.md).
