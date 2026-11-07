# elemental-core

Foundational primitives for storage, Java runtime discovery, authentication,
and process launching.

`elemental-core` contains the reusable base layer shared by the Elemental
launcher stack. It provides typed version identifiers, resource layouts,
runtime discovery and validation, authorizers, and helpers for building and
spawning launch commands. Minecraft-specific installation logic lives in
`elemental-driver`; this crate stays focused on the primitives underneath it.

## Features

- Tagged version types for Minecraft and loader version identifiers.
- Generic `Storage<L, P>` and `Layout` abstractions for nested resources.
- Java runtime discovery from platform registries, `PATH`, package-manager and
  toolchain directories, Minecraft's bundled runtimes, and `JAVA_HOME`/`JRE_HOME`.
- Explicit runtime selection with strict or disabled Java version validation.
- Offline and Microsoft device-code authorizers.
- `LaunchCommand` construction, process spawning, and streamed process logs.
- SHA-1 helpers for files and in-memory data.

## Installation

```toml
[dependencies]
anyhow = "1"
elemental-core = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

Create an offline credential and list the locally discovered Java runtimes:

```rust
use anyhow::Result;
use elemental_core::{
    auth::{authorizer::Authorizer, authorizers::offline::OfflineAuthorizer},
    minecraft::MinecraftVersionId,
    runtime::distribution::Distribution,
};

#[tokio::main]
async fn main() -> Result<()> {
    let credential = OfflineAuthorizer {
        username: "Player".to_owned(),
    }
    .authorize()
    .await?;

    println!("uuid: {}", credential.uuid);

    let game_version = MinecraftVersionId::from("1.20.1");
    println!("game version: {game_version}");

    for runtime in Distribution::cached().await.into_iter().take(5) {
        println!("{}: {}", runtime.provider, runtime.executable().display());
    }

    Ok(())
}
```

Offline credentials are intended for local development. Use an authorizer that
implements `Authorizer` for real accounts.

## Modules

| Module | Responsibility |
| --- | --- |
| `auth` | `Authorizer`, `UserCredential`, offline auth, and Microsoft device-code auth |
| `consts` | Platform-dependent constants shared by launcher code |
| `hash` | Synchronous and asynchronous SHA-1 helpers |
| `launcher` | `LaunchCommand` plus process spawning and log streaming |
| `minecraft` | Minecraft version identifiers, parsing, and semantic ordering |
| `runtime` | Java runtime providers, distributions, and validation |
| `storage` | Generic `Storage` and `Layout` resource resolution |
| `version` | The tagged `Version<Tag>` identifier wrapper |

## Core Concepts

### Typed versions

`Version<Tag>` wraps a string and carries a domain tag at the type level.
`MinecraftVersionId` uses `MinecraftVersionTag`; other crates define additional
tags such as loader versions. The wrapper is an opaque identifier and does not
define semantic ordering.

```rust
use elemental_core::minecraft::{MinecraftVersion, MinecraftVersionId};

let version = MinecraftVersionId::from("1.20.1");
assert_eq!(version.as_str(), "1.20.1");
assert_eq!(version.to_string(), "1.20.1");

let parsed = MinecraftVersion::parse("26.1").unwrap();
assert!(MinecraftVersion::parse("1.21.4").unwrap().key() < parsed.key());
```

`MinecraftVersion` parses legacy releases, year-based releases, snapshots, and
pre-release stages. Use `MinecraftVersion::key()` for sorting and range queries;
the key normalizes missing components such as the patch number in `26.1`, while
the original identifier is preserved unchanged.

### Storage and layouts

`Storage<L, P>` pairs a root path with a layout. A layout maps typed resources
to paths without hard-coding the path rules into callers.

```rust
use elemental_core::storage::{Storage, layout::Layout};

#[derive(Debug, Clone)]
struct RootLayout;

#[derive(Debug, Clone, Copy)]
enum Resource {
    Logs,
    Libraries,
}

impl Layout for RootLayout {
    type Resource = Resource;
    type ExtendedResource = Resource;

    fn get_resource(
        &self,
        root: &std::path::Path,
        resource: Self::Resource,
    ) -> Option<std::path::PathBuf> {
        Some(match resource {
            Resource::Logs => root.join("logs"),
            Resource::Libraries => root.join("libraries"),
        })
    }

    fn get_extended_resource(
        &self,
        root: &std::path::Path,
        resource: Self::ExtendedResource,
    ) -> Option<std::path::PathBuf> {
        self.get_resource(root, resource)
    }

    fn name(&self) -> &'static str {
        "root"
    }
}

let storage = Storage::new(".minecraft", RootLayout);
let logs = storage.try_get_resource(Resource::Logs)?;
# Ok::<_, anyhow::Error>(logs)
```

`Storage::scope()` nests a child layout under the current root, and
`Storage::ensure_root()` creates the directory asynchronously. `Layoutable`
adds convenience methods for reading resources through the stored layout.

### Runtime discovery

The default runtime providers inspect the Windows registry and installed
vendor directories, `PATH`, `JAVA_HOME`/`JRE_HOME`, known package-manager and
user toolchain locations (SDKMAN, asdf, Gradle, IntelliJ), and Minecraft's
bundled runtimes. Candidate roots are canonicalized, deduplicated, and sorted
by Java version. Runtimes without a `release` file are probed with a bounded
timeout and a cleaned Java environment.

```rust
use elemental_core::runtime::{
    RuntimeValidationMode, distribution::Distribution, resolve_runtime,
};

let runtime = resolve_runtime(21, None, RuntimeValidationMode::Strict, "launch").await?;
assert!(runtime.executable().is_file());
# Ok::<_, anyhow::Error>(())
```

Use `Distribution::cached()` to inspect every discovered runtime, or
`Distribution::find_cached_by_java_major()` to select one by Java major
version. An explicit executable can be passed to `resolve_runtime`; in strict
mode its detected major version must match the requested version.

Custom runtime providers can reuse the public scanning helpers in
`runtime::discovery`, including `find_runtime_homes()`,
`find_runtime_homes_with_prefixes()`, and `is_runtime_home()`, instead of
reimplementing Java runtime layout checks.

> [!IMPORTANT]
> `with_runtime_providers()` replaces the process-wide provider list. Call it
> before the first `Distribution::cached()` call, because the cached discovery
> result is not refreshed when providers change.

### Authentication and launch processes

`OfflineAuthorizer` derives a deterministic offline UUID and empty access
token. `MicrosoftAuthorizer` runs the Microsoft device-code flow and accepts a
verification callback so the host application can present the user code with
its own UI.

A `LaunchCommand` contains the runtime, structured JVM and game arguments, the
main class, environment variables, and optional working directory. It remains
mutable before launch, so callers can append authentication-specific JVM
arguments or other overrides. `spawn_command()` launches it directly, while
`spawn_command_logged()` captures stdout and stderr as `LoggedChild` lines.

## Related Crates

- [elemental-schema](https://github.com/ElectronicWave/Elemental/blob/main/crates/schema/README.md)
  supplies the manifest data models.
- [elemental-driver](https://github.com/ElectronicWave/Elemental/blob/main/crates/driver/README.md)
  builds Minecraft installations on top
  of these primitives.
- [elemental-launcher](https://github.com/ElectronicWave/Elemental/blob/main/crates/launcher/README.md)
  exposes the high-level instance lifecycle.
- [elemental-kit](https://github.com/ElectronicWave/Elemental/blob/main/README.md#the-elemental-kit-facade)
  re-exports this crate behind the `core` feature.
