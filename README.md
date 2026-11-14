# Elemental

[![Rust Test](https://github.com/ElectronicWave/Elemental/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/ElectronicWave/Elemental/actions/workflows/test.yml)
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/893a020791bf486a9ef80f67729dc2f4)](https://app.codacy.com/gh/ElectronicWave/Elemental/dashboard)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ElectronicWave/Elemental)
[![Contributors][contributors-shield]][contributors-url]
[![Stargazers][stars-shield]][stars-url]
[![License][license-shield]][license-url]

**A modular Rust toolkit for Minecraft launchers and reusable application infrastructure.**

Elemental provides a typed pipeline for discovering versions, preparing
instances, resolving Java runtimes, and launching vanilla or modded Minecraft.
It also contains standalone building blocks for version-aware persistence and
migrations, type-indexed object pools, download infrastructure, and manifest
schemas. Use `elemental-kit` end to end, or adopt the individual crates in other
Rust applications.

> [!NOTE]
> Elemental is under active development. The driver table below reflects
> current workspace coverage, not a stability guarantee. See the
> [issue tracker](https://github.com/ElectronicWave/Elemental/issues) for
> current scope and known gaps.

## Highlights

- **End-to-end lifecycle**: catalog, inspect, prepare/install, load, build a
  launch command, and launch.
- **Loader-aware design**: dedicated flows for Vanilla, Fabric-like,
  installer-driven, and profiled legacy distributions.
- **Typed storage and layouts**: game roots, instances, libraries, assets, and
  version artifacts resolve through `Storage` and `Layout`.
- **Runtime and authentication**: discover or select a local Java runtime, and
  use offline, Microsoft, or Yggdrasil accounts through one `Authorizer`
  interface.
- **Reusable infrastructure**: `elemental-shared` for versioned persistence
  and migrations, `elemental-object` for type-indexed object pools,
  `elemental-infra` for downloads, and `elemental-schema` for manifests.
- **Flexible adoption**: use the `elemental-kit` facade end to end or pick the
  individual async crates for another Rust application.

## Supported Drivers

| Family | Drivers | Lifecycle |
| --- | --- | --- |
| Vanilla | Vanilla | Catalog, inspect, install, reload, launch |
| Fabric ecosystem | Fabric, Legacy Fabric, Babric, Quilt | Catalog, inspect, install, reload, launch |
| Installer-driven | Forge, NeoForge, CleanroomMC, Outlands | Catalog, inspect, install, reload, launch |
| Profiled legacy | Rift, LiteLoader | Catalog, inspect, install, reload, launch |

These are implementation claims, not stability guarantees. Compatibility
regressions and missing coverage are tracked in the
[issue tracker](https://github.com/ElectronicWave/Elemental/issues).

## Requirements

- Rust 1.85 or newer. The workspace uses edition 2024.
- A compatible Java runtime for launching Minecraft. Elemental discovers local
  runtimes by default; use `--runtime-major-version` to select a major version,
  `--runtime-path` to add custom installation directories, or
  `RuntimeOptions::set_runtime_override` to supply a distribution directly.
- Network access for catalog and artifact downloads. Use `--local-only` when
  the instance is already prepared.

## Quick Start

```bash
git clone https://github.com/ElectronicWave/Elemental.git
cd Elemental

cargo run -p launchpad -- --help
cargo run -p launchpad -- list
```

Launchpad writes to `.minecraft` by default. Running it without launch
arguments opens the interactive wizard:

```bash
cargo run -p launchpad
```

For non-interactive launches, pass the driver id and optional versions
directly. Other drivers accept the same storage, runtime, and local-only
options:

```bash
cargo run -p launchpad -- forge 1.20.1 47.3.1
cargo run -p launchpad -- --local-only fabric 1.20.1 0.16.10 --instance-name MyFabric-1.20.1
cargo run -p launchpad -- vanilla 1.20.1 --instance-name MyVanilla-1.20.1
cargo run -p launchpad -- --runtime-major-version 21 vanilla 1.20.1
```

Launchpad defaults live in
[`crates/launchpad/src/defaults.rs`](https://github.com/ElectronicWave/Elemental/blob/main/crates/launchpad/src/defaults.rs).

## Use as a Library

```toml
[dependencies]
anyhow = "1"
elemental = { package = "elemental-kit", version = "0.3" }
tokio = { version = "1", features = ["macros", "process", "rt-multi-thread"] }
```

Prepare and launch a vanilla instance:

```rust
use anyhow::Result;
use elemental::{
    core::auth::{
        accounts::Account,
        authorizers::{Authorizer, offline::OfflineAuthorizer},
    },
    launcher::{DriverSpec, LaunchOptions, Launcher, RuntimeOptions},
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let launcher = Launcher::builder()
        .storage_root(".minecraft")
        .build();

    let prepared = launcher
        .prepare(
            "Vanilla-1.20.1".to_owned(),
            DriverSpec::vanilla("1.20.1".into()),
            &RuntimeOptions::default(),
        )
        .await?;

    let account = OfflineAuthorizer {
        username: "Player".to_owned(),
    }
    .authorize()
    .await?;
    let credential = account.credential().await?;

    let command = launcher
        .build_launch_command(
            &prepared,
            &credential,
            &RuntimeOptions::default(),
            &LaunchOptions::default(),
        )
        .await?;
    let mut game = launcher.launch(command)?;

    let status = game.child.wait().await?;
    println!("Minecraft exited with {status}");
    Ok(())
}
```

Offline authentication keeps local development simple. Use a production
`Authorizer` for real player accounts. Authentication and launch are separate:
obtain an `Account`, derive a `Credential`, build a `LaunchCommand`, optionally
customize its arguments, then spawn it. Use `prepare_with_session()` to supply a
configured session while the launcher finishes and validates it.

### Authentication and accounts

`OfflineAuthorizer`, `MicrosoftAuthorizer`, and `YggdrasilAuthorizer` implement
the same `Authorizer` contract. `AccountStore` persists accounts and refreshes
them before a credential is used. Microsoft and Yggdrasil accounts also expose
skin and cape reads and, when supported by the provider, uploads and cape
selection.

Enable `yggdrasil-auth` to install and inject authlib-injector for
authlib-compatible credentials, or `offline-server` to run a local Yggdrasil
server for development. See the
[elemental-launcher README](https://github.com/ElectronicWave/Elemental/blob/main/crates/launcher/README.md#accounts)
for account-store, texture, and offline-server examples.

Inspect and reopen an existing instance without preparing it again:

```rust
let report = launcher.inspect_instances().await?;
for instance in report.instances {
    println!("{} ({})", instance.instance_name, instance.driver.driver.id);
}
for failure in report.failures {
    eprintln!("{}: {}", failure.instance_name, failure.error);
}

if let Some(instance) = launcher.inspect_instance("MyFabric-1.20.1".to_owned()).await? {
    let prepared = launcher.load_instance(instance).await?;
    println!("Loaded {}", prepared.driver().id);
}
```

## Migrating from 0.2

0.3 is a breaking release for custom drivers: `Driver` now requires
`Send + Sync` so launcher futures can be spawned on multi-threaded runtimes.
See the [0.2 to 0.3 migration guide](docs/migration/0.3.md).

## Migrating from 0.1

0.2 is a breaking release at every layer. See the
[0.1 to 0.2 migration guide](docs/migration/0.2.md) for package, core,
authentication, launcher, downloader, persistence, object, and schema changes.

## Workspace

| Crate | Purpose |
| --- | --- |
| [`elemental-kit`](https://github.com/ElectronicWave/Elemental/blob/main/README.md#the-elemental-kit-facade) | Re-export facade for the library crates |
| [`elemental-core`](https://github.com/ElectronicWave/Elemental/blob/main/crates/core/README.md) | Storage, runtime discovery, authentication, and launch primitives |
| [`elemental-driver`](https://github.com/ElectronicWave/Elemental/blob/main/crates/driver/README.md) | Distribution catalogs, families, and loader-specific logic |
| [`elemental-launcher`](https://github.com/ElectronicWave/Elemental/blob/main/crates/launcher/README.md) | High-level launcher facade and instance lifecycle |
| [`elemental-infra`](https://github.com/ElectronicWave/Elemental/blob/main/crates/infra/README.md) | Download planning, materialization, and archive utilities |
| [`elemental-shared`](https://github.com/ElectronicWave/Elemental/blob/main/crates/shared/README.md) | Version-aware persistence and migrations |
| [`elemental-object`](https://github.com/ElectronicWave/Elemental/blob/main/crates/object/README.md) | Type-indexed global and task-local object pools |
| [`elemental-schema`](https://github.com/ElectronicWave/Elemental/blob/main/crates/schema/README.md) | Mojang and modloader manifest types |
| [`launchpad`](https://github.com/ElectronicWave/Elemental/blob/main/crates/launchpad/README.md) | End-to-end CLI launcher |

### The elemental-kit facade

`elemental-kit` re-exports the library crates under stable module names. It is
the single dependency for applications that want the launcher stack and the
reusable infrastructure together.

| Module | Package | Feature |
| --- | --- | --- |
| `elemental::core` | `elemental-core` | `core` |
| `elemental::driver` | `elemental-driver` | `driver` |
| `elemental::launcher` | `elemental-launcher` | `launcher` |
| `elemental::infra` | `elemental-infra` | `infra` |
| `elemental::object` | `elemental-object` | `object` |
| `elemental::shared` | `elemental-shared` | `shared` |
| `elemental::schema` | `elemental-schema` | `schema` |

`full` enables every module. The default feature set is `full` plus
`shared-json` and `object-notify`.

Common launcher types are available through the prelude:

```rust
use elemental::prelude::*;
```

The prelude only re-exports types from enabled modules, so it works with a
selective feature set.

For a smaller dependency surface, disable default features and select only the
modules an application uses:

```toml
[dependencies]
elemental = {
    package = "elemental-kit",
    version = "0.3",
    default-features = false,
    features = ["launcher"],
}
```

Forwarded features:

| Feature | Effect |
| --- | --- |
| `shared-json` | Enables `elemental-shared/json` |
| `shared-toml` | Enables `elemental-shared/toml` |
| `infra-serde` | Enables `elemental-infra/serde` |
| `object-notify` | Enables `elemental-object/notify` |
| `yggdrasil-auth` | Enables Yggdrasil authentication support, including authlib-injector download and launch injection |
| `authlib-injector-embed` | Embeds an authlib-injector snapshot as a fallback when metadata retrieval, download, or verification fails |
| `offline-server` | Enables the local offline Yggdrasil server and authlib-injector support in `elemental-launcher` |

> [!IMPORTANT]
> Forwarded features enable their parent module where applicable:
> `shared-json` and `shared-toml` enable `shared`, `object-notify` enables
> `object`, `infra-serde` enables `infra`, and `yggdrasil-auth` enables `core`
> and `launcher`. With `default-features = false`, add the module feature
> directly when you need it without a forwarded feature.

## Documentation

- [API documentation](https://docs.rs/elemental-kit)
- [Persistence and migration design](https://github.com/ElectronicWave/Elemental/blob/main/crates/shared/DESIGN.md)
- [Object pool design](https://github.com/ElectronicWave/Elemental/blob/main/crates/object/DESIGN.md)

## Credits

- [MultiMC](https://multimc.org/)
- [Prism Launcher](https://prismlauncher.org/)
- [XMCL](https://xmcl.app/)

<!-- LINKS -->
[contributors-shield]: https://img.shields.io/github/contributors/ElectronicWave/Elemental.svg?style=for-the-badge
[contributors-url]: https://github.com/ElectronicWave/Elemental/graphs/contributors
[stars-shield]: https://img.shields.io/github/stars/ElectronicWave/Elemental.svg?style=for-the-badge
[stars-url]: https://github.com/ElectronicWave/Elemental/stargazers
[license-shield]: https://img.shields.io/github/license/ElectronicWave/Elemental.svg?style=for-the-badge
[license-url]: https://github.com/ElectronicWave/Elemental/blob/main/LICENSE
