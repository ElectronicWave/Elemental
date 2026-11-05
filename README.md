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
> Elemental is under active development. The support matrix below reflects
> current workspace coverage, not a stability guarantee. See
> [ROADMAP.md](https://github.com/ElectronicWave/Elemental/blob/main/ROADMAP.md)
> for current scope and known gaps.

## Highlights

- **End-to-end lifecycle**: catalog, inspect, prepare/install, load, build a
  launch command, and launch.
- **Loader-aware design**: dedicated flows for Vanilla, Fabric-like,
  installer-driven, and profiled legacy distributions.
- **Typed storage and layouts**: game roots, instances, libraries, assets, and
  version artifacts resolve through `Storage` and `Layout`.
- **Runtime validation**: use an explicit Java executable or discover a
  compatible local runtime from the Minecraft version metadata.
- **Reusable infrastructure**: `elemental-shared` for versioned persistence
  and migrations, `elemental-object` for type-indexed object pools,
  `elemental-infra` for downloads, and `elemental-schema` for manifests.
- **Flexible adoption**: use the `elemental-kit` facade end to end or pick the
  individual async crates for another Rust application.

## Support Matrix

| Family | Drivers                                | Catalog | Inspect | Install | Load installed | Launch |
| --- |----------------------------------------| :---: | :---: | :---: | :---: | :---: |
| Vanilla | Vanilla                                | Yes | Yes | Yes | Yes | Yes |
| Fabric ecosystem | Fabric, Legacy Fabric, Babric, Quilt   | Yes | Yes | Yes | Yes | Yes |
| Installer-driven | Forge, NeoForge, CleanroomMC, Outlands | Yes | Yes | Yes | Yes | Yes |
| Profiled legacy | Rift, LiteLoader                       | Yes | Yes | Yes | Yes | Yes |

## Requirements

- Rust 1.85 or newer. The workspace uses edition 2024.
- A compatible Java runtime for launching Minecraft. Elemental can discover a
  local runtime or use an executable supplied with `--runtime-executable`.
- Network access for catalog and artifact downloads. Use `--local-only` when
  the instance is already prepared.

## Quick Start

```bash
git clone https://github.com/ElectronicWave/Elemental.git
cd Elemental

cargo run -p demo -- --help
cargo run -p demo -- list
```

The demo writes to `.minecraft` by default. With a compatible Java runtime
available, the default command prepares and launches Fabric 1.20.1:

```bash
cargo run -p demo
```

Other drivers accept the same storage, runtime, and local-only options:

```bash
cargo run -p demo -- forge 1.20.1 47.3.1
cargo run -p demo -- --local-only fabric 1.20.1 0.16.10 MyFabric-1.20.1
```

The demo defaults live in
[`crates/demo/src/cli.rs`](https://github.com/ElectronicWave/Elemental/blob/main/crates/demo/src/cli.rs).

## Use as a Library

```toml
[dependencies]
anyhow = "1"
elemental = { package = "elemental-kit", version = "0.2" }
tokio = { version = "1", features = ["macros", "process", "rt-multi-thread"] }
```

Prepare and launch a vanilla instance:

```rust
use anyhow::Result;
use elemental::{
    core::auth::authorizers::offline::OfflineAuthorizer,
    launcher::{DriverSpec, LaunchOptions, Launcher, PrepareInstanceRequest, VanillaSpec},
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let launcher = Launcher::builder()
        .storage_root(".minecraft")
        .build();
    let session = launcher
        .downloader()
        .session("prepare Vanilla-1.20.1")
        .open()
        .await?;

    let prepared = launcher
        .prepare_instance(
            &session,
            PrepareInstanceRequest {
                instance_name: "Vanilla-1.20.1".to_owned(),
                driver: DriverSpec::Vanilla(VanillaSpec {
                    game_version: "1.20.1".into(),
                }),
            },
        )
        .await?;
    let report = session.finish().await?;
    report.success()?;

    let mut game = launcher
        .launch_prepared_instance(
            &prepared,
            OfflineAuthorizer {
                username: "Player".to_owned(),
            },
            &LaunchOptions::default(),
        )
        .await?;

    let status = game.child.wait().await?;
    println!("Minecraft exited with {status}");
    Ok(())
}
```

Offline authentication keeps local development simple. Use a production
`Authorizer` for real player accounts. When the host application owns process
creation, call `build_launch_command` instead of `launch_prepared_instance`.

Inspect and reopen an existing instance without preparing it again:

```rust
let instances = launcher.inspect_instances().await?;
for instance in instances {
    println!("{} ({})", instance.instance_name, instance.driver.driver.id);
}

if let Some(instance) = launcher.inspect_instance("MyFabric-1.20.1".to_owned()).await? {
    let prepared = launcher.load_instance(instance).await?;
    println!("Loaded {}", prepared.driver().id);
}
```

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
| [`demo`](https://github.com/ElectronicWave/Elemental/blob/main/crates/demo/README.md) | End-to-end CLI example |

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

For a smaller dependency surface, disable default features and select only the
modules an application uses:

```toml
[dependencies]
elemental = {
    package = "elemental-kit",
    version = "0.2",
    default-features = false,
    features = ["core", "driver", "launcher"],
}
```

Forwarded features:

| Feature | Effect |
| --- | --- |
| `shared-json` | Enables `elemental-shared/json` |
| `shared-toml` | Enables `elemental-shared/toml` |
| `infra-serde` | Enables `elemental-infra/serde` |
| `object-notify` | Enables `elemental-object/notify` |

> [!IMPORTANT]
> Forwarded features do not enable the facade module itself. With
> `default-features = false`, enable both `shared` and `shared-json` when using
> JSON persistence, `shared` and `shared-toml` for TOML, and `object` plus
> `object-notify` when waiter support is required. Enable both `infra` and
> `infra-serde` when downloader events or reports need serialization.

## Documentation

- [Roadmap and current gaps](https://github.com/ElectronicWave/Elemental/blob/main/ROADMAP.md)
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
