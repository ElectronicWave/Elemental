# elemental-shared

Version-aware async persistence with explicit migrations.

`elemental-shared` keeps storage, migration, and runtime state separate. Stored
data can use an old shape, or an enum of every supported shape, while the
application works with one current type. The loader handles version validation,
migration, runtime normalization, and save decisions.

## Why elemental-shared

| Concern | Type |
| --- | --- |
| Read a schema version | `Versioned` |
| Normalize a runtime value to a version | `VersionControlled` |
| Convert between stored and current representations | `Migrator<Stored, Current>` |
| Load and save one representation | `Persistor<V>` |
| Coordinate versions, migration, and persistence | `Loader<M, S, V, P>` |

The stored and current representations may be different types. Stored V1, V2,
and V3 payloads can therefore migrate into the same current runtime type,
without the persistor needing to understand migrations.

## Features

- Cross-type migrations in both directions.
- Same-type migrations through `NoMigrator`.
- Async access through `Loader`.
- JSON and TOML file persistence.
- In-memory and no-op persistors.
- Typed errors through `thiserror`.
- Cross-platform storage locations through `Scope`.

## Installation

```toml
[dependencies]
anyhow = "1"
elemental-shared = "0.2"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

`json` is enabled by default. Enable TOML persistence explicitly:

```toml
[dependencies]
elemental-shared = { version = "0.2", features = ["toml"] }
```

## Quick Start

Use `NoMigrator` when the stored data already has the current shape.

```rust
use elemental_shared::{
    migrator::NoMigrator,
    persistor::json_persistor,
    profile::Profile,
    scope::Scope,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct GameConfig {
    player_name: String,
    level: u32,
    score: u64,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            player_name: "Player".to_owned(),
            level: 1,
            score: 0,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let loader = Profile::<GameConfig>::load(
        NoMigrator,
        json_persistor("game_save".to_owned(), Scope::Document),
        1,
    )
    .await?;

    let player_name = loader
        .get(|profile| profile.config.player_name.clone())
        .await;
    println!("Welcome, {player_name}!");

    loader
        .set(|profile| {
            profile.config.level += 1;
            profile.config.score += 100;
        })
        .await?;

    Ok(())
}
```

The final argument to `Profile::load` is the current schema version. The loader
stamps that version on the runtime value and validates the version encoded by
the migrator before saving it.

Missing storage creates `GameConfig::default()` at the target version. The
default is written only when `set()` is called.

## Cross-Type Migration

Use an enum when old configs and the current config have different shapes.
`Profile<Versions>` is the stored representation; the runtime value stays
clean.

```rust
use elemental_shared::{
    error::Result,
    migrator::Migrator,
    persistor::json_persistor,
    profile::Profile,
    scope::Scope,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct V1Config {
    player_name: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct V2Config {
    display_name: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct V3Config {
    display_name: String,
    level: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum Versions {
    V1(V1Config),
    V2(V2Config),
    V3(V3Config),
}

struct ProfileMigrator;

impl Migrator<Profile<Versions>, Profile<V3Config>> for ProfileMigrator {
    fn migrate(&self, stored: Profile<Versions>) -> Result<Profile<V3Config>> {
        let Profile { config, version } = stored;
        let config = match config {
            Versions::V1(config) => V3Config {
                display_name: config.player_name,
                level: 1,
            },
            Versions::V2(config) => V3Config {
                display_name: config.display_name,
                level: 1,
            },
            Versions::V3(config) => return Ok(Profile { config, version }),
        };

        Ok(Profile { config, version })
    }

    fn encode(&self, current: &Profile<V3Config>) -> Result<Profile<Versions>> {
        Ok(Profile {
            config: Versions::V3(current.config.clone()),
            version: current.version,
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let loader = Profile::<V3Config>::load(
        ProfileMigrator,
        json_persistor("profile".to_owned(), Scope::Config),
        3,
    )
    .await?;

    let display_name = loader
        .get(|profile| profile.config.display_name.clone())
        .await;
    println!("Welcome back, {display_name}!");

    Ok(())
}
```

The migrator converts the stored shape and may preserve the version it read.
The loader stamps the runtime value with the target version. `encode` must
return a stored value with the target version; otherwise the loader returns
`Error::EncodedVersionMismatch`.

`#[serde(deny_unknown_fields)]` keeps the untagged enum unambiguous when a newer
version adds fields. If two versions have exactly the same shape, use an
explicit tagged envelope instead of relying on untagged deserialization.

## Version Behavior

| Stored state | Loader behavior |
| --- | --- |
| Missing | Create the default current value at the target version; do not save |
| Version newer than target | Return an error |
| Version equal to target | Migrate to the current type; do not rewrite |
| Version older than target | Migrate, encode, validate, and save |

`Loader::set()` encodes the current value, validates the encoded version, and
saves it. The write lock is held through encoding and saving.

> [!WARNING]
> A slow serialization or storage backend blocks readers while `set()` holds
> the write lock. Keep persistence calls reasonably short or move them behind a
> dedicated persistor.

## Persistors

| API | Feature | Behavior |
| --- | --- | --- |
| `json_persistor` | `json` (default) | Reads and writes JSON files |
| `toml_persistor` | `toml` | Reads and writes TOML files |
| `NoPersistor` | Always | Always loads `None` and discards saves |
| `MemoryPersistor` | Always | Stores a shared in-memory value |

JSON and TOML persistors use `CustomPersistor` and `PersistorIO` internally.
Implement `Persistor<V>` directly for databases, network storage, encryption,
or other backends.

`MemoryPersistor` is useful for tests, previews, and temporary workflows:

```rust
use elemental_shared::{persistor::MemoryPersistor, version::Persistor};

let persistor = MemoryPersistor::new(Some(initial_value));
let observer = persistor.clone();

let saved = observer.load().await?;
```

Cloned handles share the same value. This is not durable storage.

## Storage Scopes

File persistors resolve paths as `<base>/.elemental/<id>.<suffix>`.

| Scope | Base directory |
| --- | --- |
| `Scope::Document` | User documents |
| `Scope::Home` | User home directory |
| `Scope::Config` | Platform config directory |
| `Scope::ConfigLocal` | Platform local config directory |
| `Scope::Dot` | Current directory |
| `Scope::Directory(path)` | Supplied directory |

For example, `Scope::Config` with the id `profile` and the default JSON suffix
produces a path such as `~/.config/.elemental/profile.json` on Linux.

## API Summary

### Core traits

```rust
pub trait Versioned {
    fn version(&self) -> usize;
}

pub trait VersionControlled: Versioned {
    fn with_version(self, version: usize) -> Self
    where
        Self: Sized;
}

pub trait Migrator<Stored: Versioned, Current: VersionControlled> {
    fn migrate(&self, stored: Stored) -> Result<Current>;
    fn encode(&self, current: &Current) -> Result<Stored>;
}

pub trait Persistor<V> {
    fn save(&self, value: &V) -> impl Future<Output = Result<()>>;
    fn load(&self) -> impl Future<Output = Result<Option<V>>>
    where
        Self: Sized;
}
```

### Loader

```rust
Loader::load(migrator, persistor, target_version).await?;
loader.get(|value| projection).await;
loader.set(|value| mutation).await?;
loader.cloned().await;
```

`Loader` holds `Arc<RwLock<V>>` internally. `get` and `cloned` take a read
lock; `set` takes a write lock. A loader can be cloned when its migrator and
persistor are cloneable.

### Versioned wrappers

`Profile<C>` stores a versioned application config:

```rust
pub struct Profile<C> {
    pub config: C,
    pub version: usize,
}
```

`Store<V>` is the generic equivalent:

```rust
pub struct Store<V> {
    pub value: V,
    pub version: usize,
}
```

Both provide a `load` helper that accepts a migrator, persistor, and target
version.

## Error Handling

All fallible APIs return `elemental_shared::error::Result<T>`. The error enum
covers:

- Filesystem I/O.
- JSON or TOML serialization and deserialization.
- Missing platform base paths.
- Stored versions newer than the supported target.
- Mismatches between encoded and target versions.

## Testing

Run the crate tests:

```bash
cargo test -p elemental-shared
```

The tests cover no-op loading and a V1/V2/V3 cross-type migration, including
verification that upgraded values are persisted through `MemoryPersistor`.

## Related Crates

- `elemental-core`
- `elemental-driver`
- `elemental-kit`

For the full design rationale, see
[DESIGN.md](https://github.com/ElectronicWave/Elemental/blob/main/crates/shared/DESIGN.md).
