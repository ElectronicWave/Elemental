# Elemental Shared Library - Design Document

## Overview

`elemental-shared` is a Rust library that provides a robust, extensible framework for managing persistent application profiles and configurations. It implements a plugin-based architecture for serialization formats and includes built-in version control and data migration capabilities.

## Core Architecture

### 1. Version Control System

**Module**: `version.rs`

The library is built around three foundational traits:

#### `VersionControlled` Trait
```rust
pub trait VersionControlled: Default {
    fn version(&self) -> usize;
    fn is_up_to_date(&self, latest_version: usize) -> bool;
}
```

- Defines the versioning contract for all managed data structures
- Allows libraries to track which version of a data structure is currently loaded
- The `is_up_to_date()` method can be overridden for custom version comparison logic

#### `Migrator<V>` Trait
```rust
pub trait Migrator<V: VersionControlled> {
    fn migrate(&self, value: V, target_version: usize) -> Result<V>;
}
```

- Responsible for upgrading data from old versions to new versions
- Implementations must handle breaking changes between versions
- Enables seamless schema evolution without data loss

#### `Persistor<V>` Trait
```rust
pub trait Persistor<V: VersionControlled> {
    async fn save(&self, value: &V) -> Result<()>;
    async fn load(&self) -> Result<Option<V>>;
}
```

- Handles all I/O operations for persisting data
- Async-first design for non-blocking file operations
- Returns `Option<V>` to gracefully handle missing files

### 2. Profile Management

**Module**: `profile.rs`

The `Profile<C>` struct encapsulates a complete user configuration:

```rust
pub struct Profile<C> {
    pub name: String,           // Profile identifier
    pub config: C,              // User's custom configuration
    pub version: usize,         // Schema version
}
```

**Key Design Decisions**:
- Generic over the configuration type `C` to support any serializable data
- Automatically implements `VersionControlled` when `C: Default`
- Provides a static `load()` method that orchestrates the full loading pipeline

**Load Pipeline**:
1. Persistor attempts to load data from storage
2. If loaded data is outdated, Migrator updates it
3. Migrated data is saved back to disk
4. Final result is wrapped in an async-aware `Loader`

### 3. Data Loading and Access

**Module**: `loader.rs`

The `Loader<M, V, P>` struct manages runtime access to persisted data:

```rust
pub struct Loader<M: Migrator<V>, V: VersionControlled, P: Persistor<V>> {
    pub inner: Arc<RwLock<V>>,  // Thread-safe, async-aware storage
    pub migrator: M,             // Migration handler
    pub persistor: P,            // Persistence handler
}
```

**Key Features**:
- **Thread-Safety**: Uses `Arc<RwLock<V>>` for safe concurrent access
- **Async Operations**: All operations are `async` and non-blocking
- **Lazy Persistence**: Changes only persisted when `set()` is explicitly called

**Public Methods**:

- `load()`: Creates and initializes a Loader instance
- `get<T>()`: Immutable read access with custom projection function
- `set()`: Mutable access with automatic persistence
- `cloned()`: Returns a clone of the entire inner value (when `V: Clone`)

### 4. Persistence Layer

**Module**: `persistor.rs`

Implements multiple layers of abstraction for storage flexibility:

#### `PersistorIO<S>` Trait
- Defines low-level I/O operations for arbitrary serialized types `S`
- `AsyncFileStringIO`: Reference implementation using `tokio::fs`

#### `CustomPersistor<V, S, IO, Ser, De>`
- Generic persistor supporting custom serialization/deserialization functions
- Separates concerns: I/O protocol, serialization format, file location
- Parameters:
  - `V`: Type to persist
  - `S`: Serialized representation
  - `IO`: I/O implementation
  - `Ser`: Serialization closure
  - `De`: Deserialization closure

#### `NoPersistor`
- Stub implementation that always loads defaults and discards saves
- Useful for testing and in-memory-only scenarios

#### Built-in Helpers
- `json_persistor()`: Provides JSON serialization (requires `json` feature)
- `toml_persistor()`: Provides TOML serialization (requires `toml` feature)

### 5. File Location Management

**Module**: `scope.rs`

The `Scope` enum abstracts file location resolution:

```rust
pub enum Scope {
    Document,                  // User's documents directory
    Home,                      // User's home directory
    Config,                    // System config directory (~/.config)
    ConfigLocal,              // Local config directory
    Dot,                      // Current working directory
    Custom(PathBuf),          // Arbitrary custom path
}
```

**Features**:
- Cross-platform path resolution using `dirs` crate
- Automatic `.elemental/` directory creation in the target location
- `get_full_path()` returns complete path with appropriate file extensions

**Design Rationale**:
- Abstracts away platform-specific directory conventions
- Centralizes file location logic for easier testing and configuration

## Data Flow

```
┌─────────────────────────────────────────────────────────┐
│ Profile::load(migrator, persistor, version)             │
└──────────────────┬──────────────────────────────────────┘
                   │
                   ▼
        ┌──────────────────────┐
        │ Persistor::load()    │
        └──────────┬───────────┘
                   │
        ┌──────────▼────────────┐
        │ Check version         │
        │ is_up_to_date()?      │
        └──────┬────────┬───────┘
               │yes     │no
               │        ▼
               │   ┌─────────────────┐
               │   │ Migrator::      │
               │   │ migrate()       │
               │   └────────┬────────┘
               │            │
               │            ▼
               │   ┌─────────────────┐
               │   │ Persistor::     │
               │   │ save()          │
               │   └────────┬────────┘
               │            │
               └────┬───────┘
                    │
                    ▼
        ┌───────────────────────┐
        │ Loader<M, V, P>       │
        │ - inner: Arc<RwLock>  │
        │ - migrator            │
        │ - persistor           │
        └───────────────────────┘
```

## Type Safety and Generics

The library leverages Rust's type system:

1. **Trait-Based Composition**: No inheritance; behaviors are composed via traits
2. **Generic Configuration**: Support for any serializable user configuration type
3. **Compile-Time Selection**: Features (`json`, `toml`) determined at compile time
4. **Zero-Cost Abstractions**: Trait objects are only used where necessary

## Concurrency Model

- **Arc<RwLock<V>>**: Allows multiple concurrent readers, single writer
- **Async/Await**: Non-blocking I/O throughout
- **Thread-Safe**: Safe to share `Loader` instances across threads/tasks
- **No Locks During I/O**: Locks released immediately after data modification

## Extension Points

1. **Custom Migrators**: Implement `Migrator<V>` for version upgrade logic
2. **Custom Persistors**: Implement `Persistor<V>` for alternative storage (databases, network, etc.)
3. **Custom Scopes**: Extend `Scope::Custom(PathBuf)` for arbitrary locations
4. **Custom Serialization**: Implement `PersistorIO<S>` for other I/O backends

## Feature Flags

- `json` (default): Enables JSON serialization support
- `toml`: Enables TOML serialization support

## Limitations and Considerations

1. **No Built-in Encryption**: Data is stored in plaintext; encryption must be implemented separately
2. **No Concurrent Writers**: Only one task can modify data at a time (enforced by `RwLock`)
3. **Synchronous File Operations**: The `Persistor` trait is async, but underlying I/O may block on some systems
4. **Migration is Manual**: No automatic schema discovery; migrations must be explicitly coded

## Testing Strategy

The library includes integration tests demonstrating:
- Profile loading with default configuration
- Configuration mutation and persistence
- Round-trip serialization (save → load → verify)
- Integration with JSON persistor and custom scopes
