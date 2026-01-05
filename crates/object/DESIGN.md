# Object Pool - Design Document

## Overview

The **elemental-object** crate provides a thread-safe, type-aware object pool for managing long-lived application objects. It enables dynamic registration, retrieval, and lifecycle management of heterogeneous types through a global singleton pool, with support for custom shutdown handlers.

## Architecture

### Core Components

#### 1. ObjectPool
The central data structure that stores all objects in a type-indexed HashMap.

```
ObjectPool
├── inner: HashMap<TypeId, PoolEntry<dyn Any + Send + Sync>>
│   └── TypeId → Unique type identifier (from std::any::TypeId)
│   └── PoolEntry → Container for the actual value + metadata
```

**Key Characteristics:**
- **Type-safe**: Uses `TypeId` to uniquely identify each type in the pool
- **Concurrency**: Built on `scc::HashMap` for lock-free concurrent reads/writes
- **Heterogeneous**: Can store values of different types in a single pool
- **Singleton**: Wrapped in `LazyLock<POOL>` for global access

#### 2. PoolEntry<T>
Represents a single entry in the pool, holding the actual value and associated metadata.

```rust
pub struct PoolEntry<T: Any + Send + Sync + ?Sized> {
    value: Option<Arc<T>>,
    notify: Arc<Notify>,              // Feature: "notify"
    shutdown: Option<ShutdownFn<T>>,
}
```

**Fields:**
- `value`: The actual object wrapped in `Arc` for shared ownership
- `notify`: (Optional) Tokio notification primitive for async waiting
- `shutdown`: Custom cleanup function executed when the value is removed

#### 3. ShutdownFn<T>
Type alias for custom lifecycle handlers.

```rust
type ShutdownFn<T> = Box<dyn Fn(Arc<T>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
```

Allows code to perform asynchronous cleanup when an object is removed from the pool.

### Global Singleton

The pool is initialized as a `LazyLock<ObjectPool>`:

```rust
static POOL: LazyLock<ObjectPool> = LazyLock::new(|| ObjectPool::new());
```

This ensures:
- Single instance across the entire application
- Thread-safe initialization (happens exactly once on first access)
- Zero-cost abstraction (no runtime allocation for uninitialized pools)

## API Layers

### Low-Level API (ObjectPool methods)

These methods operate directly on the pool instance:

| Method | Mode | Purpose |
|--------|------|---------|
| `get_sync<T>()` | Sync | Retrieve value synchronously (may be None) |
| `get_async<T>()` | Async | Retrieve value asynchronously (may be None) |
| `set_value<T>()` | Async | Store/update value with optional shutdown handler |
| `remove_value<T>()` | Async | Remove value and execute shutdown handler |
| `remove_entry<T>()` | Async | Remove entire entry and execute shutdown handler |
| `wait_value<T>()` | Async | Wait for value to be provided (requires "notify" feature) |
| `fulfill_value<T>()` | Async | Mark value as fulfilled and notify waiters |
| `shutdown()` | Async | Gracefully shutdown all objects in pool |

### High-Level API (Module-level functions)

Convenient wrappers for common operations:

**Synchronous Operations:**
```rust
require_sync<T>() -> Result<Arc<T>>
```
Get value or return error if not found.

**Asynchronous Retrieval:**
```rust
require<T>() -> Result<Arc<T>>          // Get or error
acquire<T>() -> Result<Arc<T>>          // Wait for value then get (with "notify")
```

**Value Provisioning:**
```rust
// Provide owned value
provide<T, F, Fut>(value: T, shutdown: Option<F>)

// Provide Arc-wrapped value (avoid re-wrapping)
provide_arc<T, F, Fut>(value: Arc<T>, shutdown: Option<F>)

// Mark as fulfilled (idempotent, notifies waiters with "notify")
fulfill<T>(value: T)
fulfill_arc<T>(value: Arc<T>)
```

**Value Removal:**
```rust
drop_value<T>()         // Remove and shutdown value
drop_entry<T>()         // Remove entire entry and shutdown
```

**Lifecycle:**
```rust
shutdown()              // Gracefully shutdown entire pool
```

## Design Patterns & Behaviors

### 1. Dual Modes: Wait vs. Try-Get

**Try-Get Mode** (default):
- `require()`, `require_sync()`: Return error immediately if not found
- Use when value must exist or graceful error handling is needed

**Wait Mode** (requires "notify" feature):
- `acquire()`: Blocks until value is provided
- Useful for initialization patterns: one task provides, others wait
- Prevents the "wait for initialization" race condition

### 2. Idempotent Set & Fulfill

**Multiple provides** on same type:
```rust
provide(value1, None).await;
provide(value2, None).await; // Replaces value1, triggers shutdown
```

**Multiple fulfills** on same type:
```rust
fulfill(value1).await;
fulfill(value2).await; // Ignored! Fulfillment is idempotent
```

This prevents accidental overwriting of fulfillment state in concurrent scenarios.

### 3. Shutdown Semantics

Values are cleaned up in three scenarios:

1. **Replacement**: When `provide()` replaces an existing value
   ```rust
   provide(new_value, None).await;  // Old value shutdown called
   ```

2. **Explicit Removal**: When `drop_value()` or `drop_entry()` is called
   ```rust
   drop_value::<MyType>().await;
   ```

3. **Pool Shutdown**: When `shutdown()` is called
   ```rust
   shutdown().await;  // All values shutdown in iteration order
   ```

### 4. Arc Ownership Model

All stored values are wrapped in `Arc<T>`:
- Enables shared ownership across the application
- Automatic cleanup when last reference is dropped
- Thread-safe reference counting

Values accessed via API are `Arc<T>` clones:
```rust
let value = require::<MyType>()?;  // Arc<MyType>
let cloned = value.clone();        // Reference count incremented
drop(value);                       // Reference count decremented
```

### 5. Feature-Gated Notification

The `"notify"` feature (enabled by default) provides async waiting:

```toml
[features]
notify = ["tokio"]
default = ["notify"]
```

**With feature enabled**: 
- `acquire()`, `fulfill()`, `wait_value()` are available
- Each `PoolEntry` includes `Arc<Notify>` (minimal overhead)

**With feature disabled**:
- Only try-get operations available
- Smaller memory footprint for use cases that don't need waiting

## Concurrency Model

### Synchronization Strategy

- **Data Structure**: `scc::HashMap` (lock-free concurrent hash map)
- **Read Operations**: Lock-free via `read_sync()/read_async()`
- **Write Operations**: Atomic via `upsert_async()`, `get_async()`, `remove_async()`
- **Notification**: Tokio `Notify` for async coordination (optional)

### Thread Safety Guarantees

1. **Type Safety**: Different types in pool don't interfere
2. **Reference Safety**: Values behind `Arc` have shared ownership semantics
3. **Notification Safety**: `Notify` ensures waiters wake up after fulfillment
4. **Graceful Shutdown**: Sequential iteration during pool shutdown

### Race Conditions Handled

1. **Set-before-wait**: Entry created before wait begins → immediate return
2. **Fulfill idempotence**: Multiple fulfills don't cause confusion
3. **Concurrent shutdown**: `remove_async()` prevents double-cleanup

## Usage Patterns

### Pattern 1: Singleton Service

```rust
// Initialize at startup
provide(
    MyService::new().await,
    Some(|service| async move { service.shutdown().await })
).await;

// Access anywhere
let service = require::<MyService>()?;
```

### Pattern 2: Wait-for-Initialization

```rust
// Task A: Initialize
tokio::spawn(async {
    sleep(Duration::from_secs(2)).await;
    let config = load_config().await;
    provide(config, None).await;
});

// Task B: Wait and use
let config = acquire::<Config>()?;  // Blocks until provided
use_config(&config);
```

### Pattern 3: Replacement with Cleanup

```rust
// Database connection pool with lifecycle
provide(
    db_pool,
    Some(|pool| async move {
        pool.close().await;  // Cleanup on replacement
    })
).await;
```

### Pattern 4: Explicit Cleanup

```rust
// Temporary resource
provide(TempResource::new(), Some(|res| async move {
    res.cleanup().await;
})).await;

// Later...
drop_value::<TempResource>().await;  // Triggers cleanup
```

## Type Safety & Limits

### What Works

Any type `T` where:
- `T: Any + Send + Sync`
- `T` is concretely known at compile time

```rust
provide(42usize, None).await;
provide(String::from("hello"), None).await;
provide(MyStruct { field: 42 }, None).await;
```

### What Doesn't Work

- **Dynamic types without concrete representation**: Stored as trait objects, making downcasting ambiguous
- **Non-`Send` or non-`Sync` types**: Incompatible with concurrent access
- **Dyn traits directly**: Cannot be uniquely identified by `TypeId`

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `require_sync<T>()` | O(1) | Hash lookup, blocking |
| `require<T>()` | O(1) | Hash lookup, async-friendly |
| `provide<T>()` | O(1) amortized | Insertion/update with potential reallocation |
| `acquire<T>()` | O(1) + wait | Hash lookup + notification wait |
| `drop_value<T>()` | O(1) | Removal + shutdown execution |
| `shutdown()` | O(n) | n = number of types in pool |

**Memory:**
- Per entry: `Arc<T> + Option<ShutdownFn> + Arc<Notify>`
- No allocations when pool is unused (LazyLock)

## Configuration & Features

### Default Configuration

```toml
[features]
default = ["notify"]
```

This enables async waiting and notification capabilities.

### Minimal Configuration

To disable notification overhead:

```toml
[dependencies]
elemental-object = { version = "0.1", default-features = false }
```

Results in: try-get operations only, no Tokio dependency.

## Testing

The crate includes comprehensive async tests covering:

1. **fulfill + acquire**: Waiting for provisioned values
2. **provide + require**: Direct storage and retrieval
3. **drop_value**: Explicit removal and cleanup
4. **shutdown**: Graceful pool termination
5. **multiple types**: Heterogeneous object storage

Run tests with:
```bash
cargo test --features notify
```

## Error Handling

### Errors

- **Object not found**: `require<T>()` returns `Err` with context message
  ```
  "Cannot get object `std::string::String` from pool"
  ```

### Panics

- **Shutdown handler panic**: May panic during cleanup (rare, but possible)
- **Downcast failure**: Internal bug (should never happen with correct API usage)

## Future Extensions

Potential enhancements:

1. **Weak References**: `get_weak<T>()` for non-owning access
2. **Lazy Initialization**: Auto-initialize on first access
3. **Metrics**: Count pool hits/misses, entry lifetime
4. **Distributed Pools**: Multiple pools with cross-pool queries
5. **Object Factories**: Store providers instead of values

## Security & Safety

- **Memory Safety**: All unsafe code behind `Arc` and `downcasting` is internally verified
- **No Injection Points**: No user-controlled keys or deserialization
- **Type Isolation**: Objects of different types can't collide
- **Ownership Enforcement**: Shutdown handlers execute in Arc context, ensuring proper cleanup

## Conclusion

The elemental-object crate provides a pragmatic, type-safe global object pool for Rust applications. It balances ease-of-use (simple module-level API) with flexibility (custom shutdown handlers, wait-for-initialization pattern) while maintaining thread safety through careful use of Rust's type system and concurrent data structures.
