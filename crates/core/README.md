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
- Opt-in Yggdrasil authorizer through the `yggdrasil-auth` feature.
- Account-level skin and cape queries and mutations for Microsoft and Yggdrasil.
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
    auth::{
        accounts::Account,
        authorizers::{Authorizer, offline::OfflineAuthorizer},
    },
    minecraft::MinecraftVersionId,
    runtime::RuntimeResolver,
};

#[tokio::main]
async fn main() -> Result<()> {
    let account = OfflineAuthorizer {
        username: "Player".to_owned(),
    }
    .authorize()
    .await?;
    let credential = account.credential().await?;

    println!("uuid: {}", credential.uuid);

    let game_version = MinecraftVersionId::from("1.20.1");
    println!("game version: {game_version}");

    let resolver = RuntimeResolver::with_default_providers();
    for runtime in resolver.distributions().await.into_iter().take(5) {
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
| `auth` | `Account`, `Authorizer`, `Credential`, offline, Microsoft, and Yggdrasil auth |
| `consts` | Platform-dependent constants shared by launcher code |
| `hash` | Synchronous and asynchronous SHA-1 helpers |
| `launcher` | `LaunchCommand` plus process spawning and log streaming |
| `minecraft` | Minecraft version identifiers, parsing, and semantic ordering |
| `runtime` | Java runtime providers, distributions, and validation |
| `storage` | Generic `Storage` and `Layout` resource resolution |
| `version` | The tagged `Version<Tag>` identifier wrapper |

### Feature Flags

| Feature | Description |
| --- | --- |
| `yggdrasil-auth` | Enables the Yggdrasil account and authorizer, and `Credential::auth_server` |
| `offline-server` | Enables `yggdrasil-auth` and re-exports nidhogg's offline server |

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

Use `RuntimeResolver` to own discovery and caching. `distributions()` returns
every discovered runtime, and `resolve()` selects one by Java major version:

```rust
use elemental_core::runtime::RuntimeResolver;

let resolver = RuntimeResolver::with_default_providers();
let runtime = resolver.resolve(21).await?;
assert!(runtime.executable().is_file());
```

Hosts can bypass discovery by setting `RuntimeOptions.runtime_override` to a
`Distribution`. In strict mode its detected major version must match the
required version; `RuntimeValidationMode::Disabled` skips that check.
`elemental-launcher` injects one resolver per `Launcher` so different hosts can
use different provider lists without sharing process-wide state.

Custom runtime providers can reuse the public scanning helpers in
`runtime::discovery`, including `find_runtime_homes()`,
`find_runtime_homes_with_prefixes()`, and `is_runtime_home()`, instead of
reimplementing Java runtime layout checks.

Provider lists and their cache belong to a `RuntimeResolver`. There is no
process-wide provider override, so independent resolvers can use different
providers without sharing state.

### Authentication and launch processes

Authentication is split into two module trees. `auth::accounts` owns the
`Account` trait, stable account identities, and the offline, Microsoft, and
Yggdrasil account types. The Yggdrasil types require the
`yggdrasil-auth` feature. `auth::authorizers` owns the `Authorizer` trait and
the corresponding login flows. An `Authorizer` is a login entry point with a
stable `id()` used for persistence and a human-readable `name()`.
`OfflineAuthorizer` derives a deterministic offline UUID,
`MicrosoftAuthorizer` runs the Microsoft device-code flow, and
`YggdrasilAuthorizer` authenticates against an authlib-compatible API. An
`Account` is the persisted result of authorization and can be refreshed; a
`Credential` is the per-launch projection passed to the game and is never
persisted.

`Account::textures()` returns the current skin, available capes, and the
texture kinds the account can upload. Microsoft accounts expose the owned cape
inventory for selection; Yggdrasil accounts report the active cape and their
`uploadableTextures` capabilities. Mutations are explicit:
`upload_skin()`, `delete_skin()`, `upload_cape()`, `delete_cape()`, and
`select_cape()`. Providers that do not support an operation return
`auth::Error::TextureUnsupported`. Offline accounts have no remote textures;
use nidhogg's offline server for local skin and cape files.

A `LaunchCommand` contains the runtime, structured JVM and game arguments, the
main class, environment variables, and optional working directory. It remains
mutable before launch, so callers can append authentication-specific JVM
arguments or other overrides. `spawn_command()` launches it directly and lets
the child inherit the parent's stdout and stderr.

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
