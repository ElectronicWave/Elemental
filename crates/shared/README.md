# Elemental Shared Library

Async, version-aware persistence for application profiles and settings.

`elemental-shared` separates five concerns:

- `Versioned` reads the schema version from a value.
- `VersionControlled` adds version normalization for runtime values.
- `Migrator<Stored, Current>` converts stored data into the current runtime type and back.
- `Persistor<Stored>` reads and writes one stored representation.
- `Loader` applies version checks, migration, normalization, and persistence.

The stored representation and current runtime type may be different. This allows
V1, V2, and V3 data to share one current type without forcing `Persistor` to know
anything about migrations.

## Features

- Cross-type migrations through `Migrator<Stored, Current>`
- Same-type migration shortcut through `NoMigrator`
- Async access through `Loader`
- JSON and TOML file persistence
- In-memory and no-op persistors
- Typed errors through `thiserror`
- Cross-platform storage locations through `Scope`

## Installation

```toml
[dependencies]
elemental-shared = { path = "crates/shared" }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
anyhow = "1"
```

Optional features:

```toml
[dependencies]
elemental-shared = { path = "crates/shared", features = ["json", "toml"] }
```

- `json` is enabled by default.
- `toml` enables `toml_persistor`.

## Quick Start

Use `NoMigrator` when stored data already has the current shape.

```rust
use elemental_shared::{
    migrator::NoMigrator,
    persistor::json_persistor,
    profile::Profile,
    scope::Scope,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct GameConfig {
    player_name: String,
    level: u32,
    score: u64,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            player_name: "Player".to_string(),
            level: 1,
            score: 0,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let loader = Profile::load(
        NoMigrator,
        json_persistor("game_save".to_string(), Scope::Document),
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

The final argument to `Profile::load` is the current schema version. The Loader
stamps that version on the runtime value and validates the version encoded by
the migrator before saving it.

## Profiles and Stores

`Profile<C>` is intended for profile-like data:

```rust
pub struct Profile<C> {
    pub config: C,
    pub version: usize,
}
```

`Store<V>` is a generic versioned wrapper:

```rust
pub struct Store<V> {
    pub value: V,
    pub version: usize,
}
```

Both provide a `load` helper that accepts a migrator, persistor, and target
version.

## Cross-Type Migration

Use an enum when old configs and the current config have different shapes.
`Profile<Versions>` is the stored representation; the current runtime value
stays clean.

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
struct V1Config {
    player_name: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct V2Config {
    display_name: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
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
```

Load the current type with a persistor for the stored representation:

```rust
let loader = Profile::<V3Config>::load(
    ProfileMigrator,
    json_persistor("profile".to_string(), Scope::Config),
    3,
)
.await?;
```

The migrator does not receive or set the target version. `migrate` converts the
stored shape and may preserve the version it read; the Loader stamps the runtime
value with the target version. `encode` must return the target version, which
the Loader validates before saving.

`Profile<C>` already implements `Versioned`, so the stored enum only describes
the config shape and does not need its own version implementation.

### Version Selection

`#[serde(untagged)]` selects a variant by shape. If two config versions have
exactly the same shape, use an explicit tagged envelope instead of relying on
untagged deserialization.

## Persistors

### JSON

```rust
use elemental_shared::{persistor::json_persistor, scope::Scope};

let persistor = json_persistor("config".to_string(), Scope::Config);
```

### TOML

Enable the `toml` feature:

```rust
use elemental_shared::{persistor::toml_persistor, scope::Scope};

let persistor = toml_persistor("settings".to_string(), Scope::Home);
```

### No Persistence

`NoPersistor` always returns `None` and discards saves:

```rust
use elemental_shared::{
    migrator::NoMigrator,
    persistor::NoPersistor,
    profile::Profile,
};

let loader = Profile::<GameConfig>::load(NoMigrator, NoPersistor, 1).await?;
```

The Loader creates a default runtime value. That default is not written unless
`set()` is called.

### In-Memory Persistence

`MemoryPersistor<V>` stores a shared `Option<V>` in memory:

```rust
use elemental_shared::{persistor::MemoryPersistor, version::Persistor};

let persistor = MemoryPersistor::new(Some(initial_value));
let observer = persistor.clone();

// After a migration or set(), another handle can observe the new value.
let saved = observer.load().await?;
```

Use `MemoryPersistor::default()` for an empty store.

## Storage Scopes

```rust
use elemental_shared::scope::Scope;

Scope::Document;
Scope::Home;
Scope::Config;
Scope::ConfigLocal;
Scope::Dot;
Scope::Custom("/tmp/example".into());
```

Files are placed under a `.elemental` directory:

```text
~/.config/.elemental/my_app.json
~/Documents/.elemental/game_save.json
./.elemental/config.toml
```

## API Summary

### Errors

All fallible APIs return `elemental_shared::error::Result<T>`. The error enum
covers I/O, enabled serialization formats, missing base paths, and version
validation failures.

### Versioned

```rust
pub trait Versioned {
    fn version(&self) -> usize;
}
```

Stored representations only need this trait.

### VersionControlled

```rust
pub trait VersionControlled: Versioned {
    fn with_version(self, version: usize) -> Self
    where
        Self: Sized;
}
```

Runtime values implement this trait so the Loader can set their version.

The `version` implementation is intentionally small enough to write directly;
no derive macro is needed for stored enums.

### Migrator

```rust
pub trait Migrator<Stored: Versioned, Current: VersionControlled> {
    fn migrate(&self, stored: Stored) -> Result<Current>;
    fn encode(&self, current: &Current) -> Result<Stored>;
}
```

`NoMigrator` implements `Migrator<T, T>` for cloneable values.

### Persistor

```rust
pub trait Persistor<V> {
    fn save(&self, value: &V) -> impl Future<Output = Result<()>>;
    fn load(&self) -> impl Future<Output = Result<Option<V>>>
    where
        Self: Sized;
}
```

`Persistor` does not require `Versioned`; the Loader owns all version checks.

### Loader

```rust
pub struct Loader<M, S, V, P>
where
    M: Migrator<S, V>,
    S: Versioned,
    V: VersionControlled,
    P: Persistor<S>;
```

Important methods:

```rust
Loader::load(migrator, persistor, target_version).await?;
loader.get(|value| projection).await;
loader.set(|value| mutation).await?;
loader.cloned().await;
```

## Loader Semantics

- Missing storage creates `V::default()` with the target version.
- A stored version newer than the target is rejected.
- Older stored data is migrated, encoded again, and saved.
- Current stored data is migrated into the runtime type but is not rewritten.
- `set()` encodes before saving and validates that the stored version is current.
- `set()` holds the write lock while encoding and saving.

## Testing

Run the shared crate tests:

```sh
cargo test -p elemental-shared
```

The tests cover no-op loading and a V1/V2/V3 cross-type migration. The
migration test uses `MemoryPersistor` to verify that the upgraded value is saved.

## Related Crates

- `serde`
- `tokio`
- `dirs`
- `anyhow`

See [DESIGN.md](DESIGN.md) for the architectural rationale.
