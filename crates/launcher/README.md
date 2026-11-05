# elemental-launcher

High-level instance lifecycle for prepared Minecraft installations.

`elemental-launcher` coordinates catalogs, drivers, storage layouts, the
downloader, runtime selection, authentication, and process launch. It supports
the full range of Elemental drivers while keeping the caller-facing flow small:
inspect an instance, prepare or reload it, then build a command or start the
game.

## Features

- One facade for Vanilla, Fabric, Legacy Fabric, Babric, Quilt, LiteLoader,
  Rift, Forge, NeoForge, and Cleanroom.
- Prepare new instances or reopen installations that already exist on disk.
- Automatic driver and loader-version detection during inspection.
- Shared downloader and configurable root or instance layouts.
- Launch-command construction for hosts that manage process creation.
- Process spawning through the core launch primitives.
- Typed launch options for runtime selection, resolution, quick play, and extra
  arguments.

## Installation

```toml
[dependencies]
anyhow = "1"
elemental-core = "0.2"
elemental-launcher = "0.2"
tokio = { version = "1", features = ["macros", "process", "rt-multi-thread"] }
```

`elemental-core` is needed only for the authorizer used in the example. A real
application can supply its own `Authorizer` implementation.

## Quick Start

Prepare a Vanilla instance and build its launch command:

```rust
use anyhow::Result;
use elemental_core::auth::authorizers::offline::OfflineAuthorizer;
use elemental_launcher::{
    DriverSpec, LaunchOptions, Launcher, PrepareInstanceRequest, VanillaSpec,
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

    let command = launcher
        .build_launch_command(
            &prepared,
            OfflineAuthorizer {
                username: "Player".to_owned(),
            },
            &LaunchOptions::default(),
        )
        .await?;

    println!(
        "{} with {}",
        prepared.driver().id,
        command.runtime.executable().display()
    );
    Ok(())
}
```

`prepare_instance()` downloads and writes missing artifacts. To start the game,
pass the prepared instance and command to `launch_prepared_instance()`, or use
the returned `LaunchCommand` yourself.

The caller owns the download session. Choose its name and configure retry,
failure, parallelism, and rate-limit settings before `open()`. Pass the opened
session to `prepare_instance()`, then call `finish()` or `cancel()` yourself.

## Instance Lifecycle

| Operation | Method |
| --- | --- |
| Discover local instances | `inspect_instances()` |
| Detect one instance and its driver | `inspect_instance(name)` |
| Install or refresh an instance | `prepare_instance(session, request)` |
| Reload an existing installation | `load_instance(instance)` |
| Build a command without spawning | `build_launch_command(...)` |
| Spawn the configured Java process | `launch_prepared_instance(...)` |
| Run a catalog directly | `catalog(catalog)` |

Reopen an instance without preparing it again:

```rust
let instances = launcher.inspect_instances().await?;
for instance in instances {
    println!("{} ({})", instance.instance_name, instance.driver.driver.id);
}

if let Some(instance) = launcher
    .inspect_instance("Vanilla-1.20.1".to_owned())
    .await?
{
    let prepared = launcher.load_instance(instance).await?;
    println!("Loaded {}", prepared.driver().id);
}
```

> [!NOTE]
> `load_instance()` expects a complete local installation. Use `--local-only`
> in the demo, or inspect before loading, when network access is unavailable.

## Driver Specifications

`DriverSpec` selects the installation flow:

| Variant | Loader version |
| --- | --- |
| `Vanilla` | Game version only |
| `Fabric`, `LegacyFabric`, `Babric`, `Quilt` | Game and loader version |
| `LiteLoader`, `Rift` | Game and loader version |
| `Forge`, `NeoForge`, `Cleanroom` | Game and loader version |

Loader variants use `LoaderSpec { game_version, loader_version }`. Both version
fields accept strings through `Into`.

## Launch Options

`LaunchOptions` is an alias for `VersionJsonLaunchConfig`. It covers:

- `runtime_major_version`, `runtime_executable_path`, and
  `runtime_validation`.
- Launcher name, launcher version, and client ID.
- Window resolution and quick-play targets.
- Extra JVM and game arguments.

Use `LaunchOptions::default()` for automatic runtime selection with strict Java
version validation.

## Builder Customization

The default storage root is `.minecraft`, and the builder creates a shared
downloader automatically. Applications can override either:

```rust
use elemental_infra::downloader::ElementalDownloader;
use elemental_launcher::Launcher;

let launcher = Launcher::builder()
    .storage_root("instances")
    .downloader(ElementalDownloader::new())
    .build();
```

Use `root_layout()` and `instance_layout()` for custom layouts, or
`with_layouts()` to replace both at once. `Launcher::new(storage_root,
downloader)` is the shorthand when the base layouts are sufficient.

> [!WARNING]
> Preparing an instance may download metadata, libraries, assets, and loader
> installers. Launching it requires a compatible local Java runtime unless an
> explicit executable is supplied.

## Related Crates

- [elemental-core](https://github.com/ElectronicWave/Elemental/blob/main/crates/core/README.md)
  supplies authentication, runtime discovery, storage, and process primitives.
- [elemental-driver](https://github.com/ElectronicWave/Elemental/blob/main/crates/driver/README.md)
  supplies catalogs and loader-specific preparation logic.
- [elemental-infra](https://github.com/ElectronicWave/Elemental/blob/main/crates/infra/README.md)
  supplies the downloader.
- [elemental-kit](https://github.com/ElectronicWave/Elemental/blob/main/README.md#the-elemental-kit-facade)
  re-exports this crate behind the `launcher` feature.
- [demo](https://github.com/ElectronicWave/Elemental/blob/main/crates/demo/README.md)
  shows the complete CLI flow.
