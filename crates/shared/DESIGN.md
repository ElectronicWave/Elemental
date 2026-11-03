# Elemental Shared Library Design

## Overview

`elemental-shared` provides version-aware asynchronous persistence without
forcing stored data and runtime data to have the same type.

The design uses two representations:

- `Stored`: the versioned representation read from and written to a persistor.
- `Current`: the versioned representation used by the application at runtime.

`Migrator<Stored, Current>` crosses the type boundary in both directions.
`Persistor<Stored>` remains symmetric and only understands one representation.

## Design Principles

1. `Persistor` owns storage and serialization, not migrations.
2. `Migrator` owns representation changes in both directions.
3. `Loader` owns version checks, runtime normalization, stored-version
   validation, and save decisions.
4. The runtime type stays free of old-version variants.
5. Missing storage is represented as `None`, not as a fabricated stored value.

## Error Model

All fallible public APIs return `crate::error::Result<T>`. `Error` uses
`thiserror` so callers can match storage, serialization, path, and version
failures without downcasting:

- `Io` wraps filesystem errors.
- `Json` wraps JSON failures when the `json` feature is enabled.
- `TomlSerialize` and `TomlDeserialize` wrap TOML failures when `toml` is enabled.
- `BasePathUnavailable` reports an unsupported platform storage location.
- `StoredVersionNewer` rejects data from a newer schema.
- `EncodedVersionMismatch` rejects an encoded representation with the wrong
  version.

## Core Types

| Type | Responsibility |
| --- | --- |
| `Versioned` | Reads the schema version |
| `VersionControlled` | Adds consuming version normalization |
| `Migrator<Stored, Current>` | Converts `Stored` to `Current` and encodes it back |
| `Persistor<V>` | Loads and saves one representation |
| `Loader<M, S, V, P>` | Coordinates migration, normalization, and persistence |
| `Profile<C>` | Versioned application profile with a generic configuration |
| `Store<V>` | Generic versioned wrapper |

## Version Model

`Versioned` is the read-only contract needed by stored representations:

```rust
pub trait Versioned {
    fn version(&self) -> usize;
}
```

`VersionControlled` extends it for runtime values whose version must be
normalized:

```rust
pub trait VersionControlled: Versioned {
    fn with_version(self, version: usize) -> Self
    where
        Self: Sized;
}
```

`with_version` is consuming rather than mutating. This keeps version
normalization out of `Migrator` and avoids a separate mutable-version trait.
`Profile<C>` and `Store<V>` already implement `Versioned`, so a stored enum
only needs to describe the config shape:

```rust
#[serde(untagged)]
enum Versions {
    V1(V1Config),
    V2(V2Config),
    V3(V3Config),
}
```

`Profile<Versions>` is the complete stored representation. Its version is
stored once by `Profile`, rather than duplicated inside each enum variant.

The Loader decides which version becomes current. A migrator converts data shape
and may preserve the source version. The Loader stamps the target version on
the runtime value, then validates the stored version returned by `encode`.

## Cross-Type Migration

The migration contract handles both directions:

```rust
pub trait Migrator<Stored: Versioned, Current: VersionControlled> {
    fn migrate(&self, stored: Stored) -> Result<Current>;
    fn encode(&self, current: &Current) -> Result<Stored>;
}
```

`migrate` is used when loading. `encode` is used when a migrated value or a
changed current value must be written back.

The stored side can be `Profile<Versions>`, where the enum contains every
supported config shape:

```rust
#[serde(untagged)]
enum Versions {
    V1(V1Config),
    V2(V2Config),
    V3(V3Config),
}
```

This allows one `Profile<V3Config>` runtime type while the persistor remains
focused on `Profile<Versions>`.

When `Stored` and `Current` are the same type, `NoMigrator` implements
`Migrator<T, T>`:

```rust
pub struct NoMigrator;

impl<T> Migrator<T, T> for NoMigrator
where
    T: VersionControlled + Clone,
{
    // migrate returns the stored value
    // encode clones the current value
}
```

## Persistence Model

`Persistor<V>` is symmetric:

```rust
pub trait Persistor<V> {
    fn save(&self, value: &V) -> impl Future<Output = Result<()>>;
    fn load(&self) -> impl Future<Output = Result<Option<V>>>
    where
        Self: Sized;
}
```

The trait does not know about `Current`, migrations, or versions. Version checks
belong to `Loader`.

### File Persistors

`CustomPersistor<V, S, D, IO, Ser, De>` separates:

- `V`: value exposed to the Loader
- `S`: serialized representation
- `D`: borrowed input type used by deserialization
- `IO`: byte/string I/O implementation
- `Ser`: serialization function
- `De`: deserialization function

`json_persistor` and `toml_persistor` are built on this abstraction.

### NoPersistor

`NoPersistor` loads `None` and discards saves. It is useful when persistence is
intentionally disabled. The Loader still creates a current default value, but
that value is not written to storage unless `set()` is called.

### MemoryPersistor

`MemoryPersistor<V>` stores `Option<V>` in `Arc<Mutex<Option<V>>>`.

- `load` clones the current value.
- `save` replaces the current value.
- Cloned handles share the same slot.

It is suitable for tests, previews, and temporary in-memory workflows. It is not
durable storage.

## Load Flow

```text
Persistor::load()
        |
        +-- None ------------------------------+
        |                                      |
        |                             V::default()
        |                                      |
        |                            with_version(target)
        |                                      |
        +-- Some(stored)                        |
                 |                              |
          stored.version() > target?            |
                 | yes                          |
                 +--> error                     |
                 | no                           |
                 +--> Migrator::migrate()        |
                              |                 |
                       with_version(target)     |
                              |                 |
                 stored.version() < target?     |
                         | yes      | no        |
                         v          |           |
               Migrator::encode()  |           |
                         |          |           |
          stored.version() == target?          |
                         | no       |           |
                         +--> error |           |
                         | yes      |           |
               Persistor::save()   |           |
                         |          |           |
                         +----------+-----------+
                                    |
                               Loader<V>
```

Version rules:

| Stored version | Behavior |
| --- | --- |
| Greater than target | Return an error |
| Equal to target | Migrate to current, do not rewrite |
| Less than target | Migrate, encode, validate, save |
| Missing | Create default current value, do not save |

## Set Flow

```text
Loader::set(closure)
        |
        v
Acquire write lock
        |
        v
Apply closure to Current
        |
        v
Migrator::encode()
        |
        v
Stored.version() == target?
        | no
        +--> error
        | yes
        |
        v
Persistor::save()
```

The write lock is held through encoding and saving so concurrent readers do not
observe a partially saved update. Long-running serialization or I/O therefore
blocks other access to that Loader.

## Type Relationships

```text
                 +----------------------+
                 |  Migrator<S, V>      |
                 |                      |
     stored S -->| migrate              |
                 | encode               |--> stored S
                 |                      |
                 +----------+-----------+
                            |
                            v
                 +----------------------+
                 |  Loader<M, S, V, P>  |
                 |                      |
                 | version rules        |
                 | runtime normalization|
                 | stored validation    |
                 | save decisions       |
                 +----------+-----------+
                            |
                            v
                 +----------------------+
                 |  Persistor<S>        |
                 |                      |
                 | load / save          |
                 +----------------------+
```

## Concurrency

- `Loader::inner` is `Arc<RwLock<V>>`.
- Reads use a shared read lock.
- Writes use an exclusive write lock.
- `Loader` and `MemoryPersistor` can be cloned and shared.
- The Loader does not perform background writes; `set()` and load-time upgrades
  are explicit asynchronous operations.

## Scope Resolution

`Scope` resolves a base directory and appends `.elemental/<id>.<extension>`.

Supported bases:

- user documents
- home directory
- platform config directory
- platform local config directory
- current directory
- custom directory

Path creation is handled through `get_full_path` asynchronously and
`get_full_path_blocking` synchronously.

## Extension Points

### Custom Migrator

Implement `Migrator<Stored, Current>` when stored and current types differ or
when old data needs structural conversion.

### Custom Persistor

Implement `Persistor<V>` for databases, network storage, encrypted files, or
other backends. Keep migration logic outside the persistor.

### Custom Serialization

Use `CustomPersistor` or `PersistorIO` when adding a serialization format or I/O
backend.

## Tradeoffs

### Untagged Version Enums

`#[serde(untagged)]` selects by data shape. Two versions with identical shapes
cannot be distinguished by shape alone; use an explicit tagged envelope for
that case.

### Manual Migrations

Schema discovery is not automatic. Each supported old representation must be
included in `Stored` and handled by the migrator.

### Plaintext Storage

File-based persistors do not provide encryption. Encryption belongs in a
custom persistor or an additional encoding layer.

### Lock Duration

`set()` holds the write lock across encoding and persistence. This guarantees
consistent in-memory state but can reduce concurrency when saving is slow.

## Testing Strategy

The crate tests cover:

- no-op loading with `NoMigrator`
- V1/V2/V3 cross-type migration
- migration back to the stored representation
- persistence through `MemoryPersistor`

`MemoryPersistor` is deliberately shared between the Loader and the test so the
test observes the same state that was saved.
