# Object Pool - Design Document

## Overview

`elemental-object` stores one value per concrete Rust type in a concurrent,
type-indexed pool. It supports synchronous retrieval, async provisioning,
optional waiting, hot replacement, task-local contexts, and asynchronous
shutdown hooks.

The design optimizes for a small lifecycle surface:

- one `TypeId` maps to one `PoolEntry`;
- values are shared as `Arc<T>`;
- state changes are serialized by `scc::HashMap` entry guards, which lock the
  containing bucket internally;
- notifications and shutdown hooks run after the entry state is updated and
  after the entry lock is released.

## Architecture

### ObjectPool

```rust
pub struct ObjectPool {
    inner: HashMap<TypeId, PoolEntry<dyn Any + Send + Sync>>,
}
```

`scc::HashMap` provides concurrent lookup and per-entry exclusive access.
`TypeId` is the only key, so types with the same layout still have independent
slots.

The process-wide pool is a `LazyLock<ObjectPool>`. `ObjectContext` owns a
separate `ObjectPool` and exposes it through `tokio::task_local`.

### PoolEntry

```rust
pub struct PoolEntry<T: Any + Send + Sync + ?Sized> {
    value: Option<Arc<T>>,
    #[cfg(feature = "notify")]
    notify: Arc<Notify>,
    active: AtomicBool,
    shutdown: Option<ShutdownFn<T>>,
}
```

- `value` is the currently published object.
- `notify` wakes waiters when this entry changes. It is compiled only with the
  `notify` feature.
- `active` distinguishes a pending or ready slot from a closed slot. It is
  always present because the non-notify build still needs the same lifecycle
  semantics.
- `shutdown` is the hook associated with the value that is currently stored.

The state invariants are:

| `active` | `value` | Meaning |
| --- | --- | --- |
| `true` | `Some` | Ready |
| `true` | `None` | Pending, created by a waiter |
| `false` | `None` | Closed by `drop_value()` or `shutdown()` |

An inactive entry never has a value: the cleanup path clears both fields while
holding the entry lock.

### CleanupTask

`CleanupTask` carries the old value, its shutdown hook, and a notification
handle out of the entry lock. `run()` first notifies waiters, then awaits the
shutdown hook.

Keeping cleanup outside the lock has two purposes:

1. A slow shutdown hook cannot block readers or writers of the same entry or
   other entries in the same hash-map bucket.
2. A shutdown hook can safely call back into the pool without re-entering an
   entry lock already held by the replacement path.

## State Transitions

### provide

`set_value()` performs one `entry_async()` operation and handles both cases:

- occupied entry: replace the value and shutdown hook, then return the old
  state as a `CleanupTask`;
- vacant entry: insert a new active entry with the value.

This removes the earlier `contains` followed by `get().unwrap()` race. It also
means `provide()` reopens a slot closed by `drop_value()` or `shutdown()`.

### fulfill

`fulfill_value()` takes an exclusive entry reference and checks the state
directly:

| State | Result |
| --- | --- |
| No entry | `Err`: no waiting entry exists |
| Closed entry | `Err`: use `provide` to reopen it |
| Active with a value | `Ok(false)`: leave the existing value untouched |
| Active and empty | `Ok(true)`: fill the slot and notify waiters |

`fulfill()` is therefore not an idempotent silent no-op. It reports whether it
actually filled a slot and never revives a closed entry.

### drop_value

`remove_value()` keeps the entry but calls `take_cleanup()`:

- the value and shutdown hook are moved out;
- `active` becomes `false`;
- waiters are notified outside the lock;
- the shutdown hook, if present, is awaited outside the lock.

It returns whether a value was actually present. Waiters observe the inactive
state and return, after which `require()` reports the missing value. A later
`provide()` can reopen the same entry.

### drop_entry

`remove_entry()` removes the map entry and runs its cleanup. It returns whether
the entry existed. A waiter that was blocked on the removed entry is notified,
observes `None`, and starts another wait iteration on a newly created entry.
`drop_entry()` is therefore not a signal that all future producers are gone.

### shutdown

`shutdown()` walks the map, marks every entry inactive, and collects each
cleanup task. It then runs all shutdown hooks after the iteration, without
holding entry locks. Hooks run sequentially, but no ordering guarantee is made
across types.

## Waiting Model

`acquire()` is implemented by `wait_value()` followed by `require()`. The wait
loop uses one state snapshot per iteration:

```rust
loop {
    let entry = self
        .inner
        .entry_async(type_id)
        .await
        .or_insert_with(PoolEntry::new);
    let (notified, state) = {
        let pool_entry = entry.get();
        (
            pool_entry.notify.clone().notified_owned(),
            (pool_entry.is_active(), pool_entry.value.is_some()),
        )
    };
    drop(entry);

    match state {
        (false, _) | (true, true) => return,
        (true, false) => notified.await,
    }
}
```

The ordering is intentional:

1. Take one exclusive entry access, creating the entry if it is missing.
2. Create an owned `Notified` future before reading state.
3. Read `active` and `value` together while the same entry lock is held.
4. Drop the lock, then decide based on that single snapshot or await the
   already-created future.

Tokio's owned notification future captures the `notify_waiters()` counter at
creation, so a notification sent after creation is observed even if the future
has not been polled yet. Registering and reading under the same lock is what
ties the notification handle to the entry that produced the snapshot. Without
that, an entry could be removed and recreated between notification creation and
the state read, leaving the waiter on the old entry's notification handle. The
loop is required because an entry can still be removed after the lock is
released; removal wakes the waiter, which then creates or joins the replacement
entry.

There is no generation counter. A waiter only needs to know whether a value is
available or the slot is closed; it does not need to identify a particular
write. If a value is replaced while a waiter is blocked, the waiter wakes and
reads the current value. A generation or replay log would only be needed for a
subscription API that delivers every write, which this pool does not provide.

## Removal Races

- `provide` races are resolved by `entry_async()` serializing the occupied and
  vacant cases.
- `fulfill` checks "already filled" and "closed" under the same exclusive
  entry access used for insertion.
- Concurrent `drop_value` calls move the state out exactly once; the second
  call returns `false`.
- Concurrent `drop_entry` calls remove the entry exactly once.
- A remove operation notifies waiters after updating state, so waiters never
  block on a state they cannot observe.

## API Layers

### ObjectPool

| Method | Behavior |
| --- | --- |
| `get_async<T>()` | Return the current `Arc<T>` or `None` |
| `set_value<T>()` | Insert, replace, or reopen a value |
| `remove_value<T>()` | Close a slot; return whether a value was present |
| `remove_entry<T>()` | Remove an entry; return whether it existed |
| `wait_value<T>()` | Wait for a value or a closed slot; `notify` only |
| `fulfill_value<T>()` | Fill a pending slot; return `Result<bool>` |
| `shutdown()` | Close all entries and run their shutdown hooks |

### Facade

- Retrieval: `require()`, `acquire()` with the `notify` feature.
- Provisioning: `provide()`, `provide_with_shutdown()`, `provide_context()`,
  `provide_context_with_shutdown()`, `provide_instant()`,
  `provide_instant_context()`.
- Fulfillment: `fulfill()`, `fulfill_with_shutdown()`, `fulfill_context()`,
  `fulfill_with_context_shutdown()`; all return `Result<bool>`.
- Removal: `drop_value()`, `drop_entry()`, `drop_context_value()`,
  `drop_context_entry()`; all return `bool`.
- Shutdown: `shutdown()`, `shutdown_local()`; `shutdown_local()` returns
  whether a context was active.

`require()` checks the active context first, then the global pool. Context-aware
providers and fulfillers target the active context when present and fall back to
the global pool. Context-aware removal and `shutdown_local()` do not fall back;
they return `false` when no context is active.

## Feature Flags

`notify` is enabled by default and gates `acquire()`, `wait_value()`, and the
`Notify` field. Tokio is a regular dependency because `ObjectContext` requires
`tokio::task_local` in every configuration.

Without `notify`, provisioning, retrieval, fulfillment of existing entries,
removal, and shutdown remain available. There is no public waiter path, so a
pending empty entry is normally not created.

## Non-Goals

- **Automatic type dependency resolution.** There is no dependency graph,
  topological construction, or implicit lookup of another type during
  provisioning. Compose values explicitly or use `InstantObject`.
- **Generation-based event delivery.** `acquire()` returns the current value;
  it does not replay every update.
- **Cross-type shutdown ordering.** Hooks are independent and may not be
  ordered relative to other types.
- **Dynamic trait-object keys.** Values are identified by concrete `TypeId`.
- **Distributed pools.** The pool is process-local.

## Error Handling

- `require()` returns an `Error::ObjectNotFound` containing the requested type
  name when no value exists.
- `acquire()` waits, then returns the same error if the slot was closed or the
  value disappeared before retrieval.
- `fulfill()` returns an error for a missing entry or a closed entry and
  `Ok(false)` for an already-filled entry.
- Removal functions report absence with `false` rather than an error.

## Verification

The crate's smoke tests cover:

- provide, retrieve, and remove;
- waiting for a provisioned value;
- fulfill with and without a pending entry;
- closed-slot and removed-entry behavior;
- shutdown hooks running outside the entry lock;
- task-local contexts;
- facade-level provide, require, and drop.

Relevant commands:

```text
cargo test -p elemental-object
cargo test -p elemental-object --no-default-features
cargo check -p elemental-object --no-default-features
```
