# Contributing to Elemental

Thank you for your interest in contributing to **Elemental**.

Elemental is designed to be a **foundational toolkit**: composable like elements, minimal in assumptions, and transparent to downstream users. This document defines the contribution principles that help us keep the project maintainable, predictable, and developer-friendly over the long term.

---

## Core Philosophy

Before writing code, please align with the following principles:

### 1. Developer-Friendly by Default

* APIs should be **intuitive**, **discoverable**, and **hard to misuse**.
* Prefer explicitness over magic.
* A new contributor should be able to understand *why* a design exists, not just *how* to use it.

### 2. User Transparency

* Elemental should never surprise its users.
* Errors must be visible and explainable.
* Side effects (IO, state mutation, network calls) must be explicit.

### 3. Composability First

Elemental components should behave like elements:

* Small, single-purpose
* Freely composable
* No hidden global state
* No implicit coupling between modules

If a feature cannot be composed cleanly, it likely does not belong in the core.

---

## API Design Guidelines

### Clear and Predictable APIs

* Function names must describe **what** they do, not **how** they do it.
* Avoid overloaded behavior controlled by flags or booleans.
* Prefer:

```rust
fn load_config(path: &Path) -> Result<Config>
```

Over:

```rust
fn load(path: &Path, strict: bool, cache: bool) -> Config
```

### Prefer Types Over Conventions

* Encode invariants in types whenever possible.
* Avoid "magic strings" and loosely-typed parameters.

### Backward Compatibility

* Public APIs are treated as stable contracts.
* Breaking changes must be:

  * Explicit
  * Justified
  * Documented

If backward compatibility is not possible, provide a migration path.

---

## Error Handling

### No `panic!` in Library Code

* **Do not** use `panic!` for recoverable errors.
* Prefer `anyhow::Result<T>` or a well-defined error enum.

```rust
use anyhow::Result;

fn parse(input: &str) -> Result<Ast> {
    // ...
}
```

### When Errors Can Be Ignored

* If an error is truly non-critical:

  * Log it
  * Document why it is safe to ignore

Silent failure is not acceptable.

---

## Code Style

### Formatting

* All Rust code must be formatted with `rustfmt`.
* Do not hand-format or rely on editor-specific styles.

```sh
cargo fmt
```

### Readability Over Cleverness

* Prefer straightforward implementations.
* Avoid clever tricks that trade clarity for brevity.
* Optimize for maintainability first.

### Small, Focused Changes

* One pull request = one logical change.
* Refactors and behavior changes should not be mixed.

---

## Dependency Management

### Minimal Dependencies

* Every dependency increases maintenance cost.
* Prefer the standard library when reasonable.
* Avoid introducing dependencies for trivial functionality.

### Sorting Dependencies

Whenever you modify `Cargo.toml`, ensure dependencies are sorted:

```sh
cargo install cargo-sort
cargo sort
```

This keeps diffs clean and predictable.

---

## Documentation

### Code-Level Documentation

* Public APIs **must** have rustdoc comments.
* Document:

  * What the function does
  * Error conditions
  * Important invariants

### Design Documentation

* Non-trivial architectural decisions should be documented.
* If you had to think hard to design it, future contributors will too.

---

## Testing

* New features must include tests.
* Bug fixes must include regression tests.
* Prefer deterministic tests over timing-based or flaky tests.

If something is hard to test, that is a design signal.

---

## Pull Request Guidelines

Before opening a PR, ensure:

* [ ] Code builds and tests pass
* [ ] Code is formatted (`cargo fmt`)
* [ ] Dependencies are sorted (`cargo sort`)
* [ ] Public APIs are documented
* [ ] The change aligns with Elemental’s composability philosophy

### PR Descriptions

Explain:

* **What** problem is being solved
* **Why** this approach was chosen
* **What** trade-offs were considered

---

## What Does *Not* Belong in Elemental Core

To keep Elemental foundational:

* Opinionated application logic
* Hard-coded environment assumptions
* Feature flags that change semantics
* Framework-style lifecycle management

These belong in higher-level crates built *on top of* Elemental.

---

## Final Note

Elemental aims to be boring in the best way:

* Predictable
* Stable
* Composable

If your contribution makes the system simpler, clearer, or easier to combine with other components, it is probably a good fit.

Thank you for helping build a solid foundation.
