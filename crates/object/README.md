# elemental-object

A type-safe, thread-safe global object pool for Rust applications.

## Features

- **Type-Safe**: Leverages Rust's type system to uniquely identify and retrieve objects
- **Lock-Free**: Built on `scc::HashMap` for efficient concurrent access
- **Async-Friendly**: Full support for async/await patterns
- **Lifecycle Management**: Custom shutdown handlers for graceful cleanup
- **Wait-for-Initialization**: Optional notification system for coordinating startup
- **Zero-Cost Abstractions**: LazyLock ensures minimal overhead

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
elemental-object = "0.1"
```

Basic usage:

```rust
use elemental_object::{provide, require};

#[tokio::main]
async fn main() {
    // Store a value
    let config = MyConfig { 
        database_url: "postgres://localhost".to_string() 
    };
    provide(config, None).await;

    // Retrieve it anywhere in your app
    let config = require::<MyConfig>()?;
    println!("Database: {}", config.database_url);
    
    Ok(())
}
```

## Common Patterns

### 1. Global Singleton Service

Store and access application services without passing them through every function:

```rust
use elemental_object::{provide, require};

struct DatabaseConnection {
    // ...
}

impl DatabaseConnection {
    async fn shutdown(&self) {
        // Cleanup code
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = DatabaseConnection::new().await;
    
    // Register with shutdown handler
    provide(
        db,
        Some(|db| async move {
            db.shutdown().await;
        })
    ).await;

    // Access from anywhere
    let db = require::<DatabaseConnection>()?;
    // ... use db
    
    Ok(())
}
```

### 2. Wait-for-Initialization Pattern

Coordinate startup between multiple tasks:

```rust
use elemental_object::{provide, acquire};

#[tokio::main]
async fn main() {
    // Task 1: Initialize config from slow source
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let config = load_config_from_file().await;
        provide(config, None).await;
    });

    // Task 2: Wait until config is ready
    let config = acquire::<AppConfig>()?;  // Blocks until provided
    start_server(config).await;
}
```

### 3. Multiple Types in One Pool

Store different types without creating separate managers:

```rust
use elemental_object::{provide, require};

provide(my_db, None).await;
provide(my_cache, None).await;
provide(my_logger, None).await;

// Retrieve each by type
let db = require::<Database>()?;
let cache = require::<Cache>()?;
let logger = require::<Logger>()?;
```

### 4. Lifecycle with Cleanup

Ensure resources are properly cleaned up when replaced or removed:

```rust
use elemental_object::{provide, drop_value};

// Provide resource with shutdown handler
provide(
    resource,
    Some(|res| async move {
        println!("Cleaning up: {}", res.name);
        res.cleanup().await;
    })
).await;

// Later: explicitly remove (triggers cleanup)
drop_value::<MyResource>().await;

// Or replace with new value (triggers cleanup of old value)
provide(new_resource, None).await;
```

## API Overview

### Retrieving Values

| Function | Mode | Behavior |
|----------|------|----------|
| `require::<T>()` | Async | Get value or return error |
| `require_sync::<T>()` | Sync | Get value synchronously or return error |
| `acquire::<T>()` | Async | Wait for value to be provided, then return it |

### Providing Values

| Function | Behavior |
|----------|----------|
| `provide::<T>(value, shutdown)` | Store owned value with optional shutdown handler |
| `provide_arc::<T>(arc_value, shutdown)` | Store Arc-wrapped value (avoids re-wrapping) |
| `fulfill::<T>(value)` | Mark value as fulfilled and notify waiters (idempotent) |
| `fulfill_arc::<T>(arc_value)` | Fulfill with Arc value |

### Cleanup

| Function | Behavior |
|----------|----------|
| `drop_value::<T>()` | Remove value and call shutdown handler |
| `drop_entry::<T>()` | Remove entire entry and call shutdown handler |
| `shutdown()` | Gracefully shutdown all objects in pool |

## Features

### Default (with notification)

The default configuration includes the `"notify"` feature, enabling wait-for-initialization patterns:

```rust
use elemental_object::acquire;

let config = acquire::<AppConfig>()?;  // Available
```

### Minimal (without notification)

For applications that don't need async waiting, reduce dependencies:

```toml
[dependencies]
elemental-object = { version = "0.1", default-features = false }
```

This removes the Tokio dependency and disables `acquire()`, `fulfill()`, and `wait_value()`.

## Thread Safety

- All operations are thread-safe and concurrent
- Values are wrapped in `Arc<T>` for shared ownership
- Built on lock-free `scc::HashMap` for efficient concurrent access
- Supports both sync and async access patterns

## Type Requirements

Values stored in the pool must satisfy:

```rust
T: Any + Send + Sync
```

This includes:
- Most standard types (`String`, integers, collections)
- Custom structs and enums
- Trait objects (less common, as type identification becomes ambiguous)

## Error Handling

When a value is not found in the pool:

```rust
use elemental_object::require;

match require::<Config>() {
    Ok(config) => { /* ... */ },
    Err(e) => eprintln!("Config not initialized: {}", e),
}
```

The error message includes the type name for debugging:
```
Cannot get object `my_app::config::Config` from pool
```

## Examples

See the [tests](src/lib.rs) for more complete examples:

- `test_fulfill`: Wait-for-initialization pattern
- `test_provide`: Multiple types in pool

Run with:

```bash
cargo test --features notify
```

## Performance

- **Retrieval**: O(1) hash lookup
- **Insertion/Update**: O(1) amortized
- **Shutdown**: O(n) where n = number of types in pool
- **Memory**: Minimal overhead (LazyLock initializes on first use)

## Design Document

For detailed architectural information, see [DESIGN.md](DESIGN.md).

## Integration with Elemental

This crate is part of the [Elemental](https://github.com/ElectronicWave/Elemental) framework and is commonly used in conjunction with:

- `elemental-core`: Core framework utilities
- `elemental-shared`: Shared configuration and persistence

## License

Licensed under the same terms as the Elemental project. See [LICENSE](../../LICENSE) for details.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

---

For questions or issues, please open an issue on the [GitHub repository](https://github.com/ElectronicWave/Elemental).
