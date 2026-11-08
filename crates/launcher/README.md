# elemental-launcher

High-level instance lifecycle for prepared Minecraft installations.

`elemental-launcher` coordinates catalogs, drivers, storage layouts, the
downloader, runtime selection, and process launch. Authentication is completed
by the caller before launch. It supports the full range of Elemental drivers
while keeping the caller-facing flow small: inspect an instance, prepare or
reload it, build a mutable launch command, then start the game.

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
use elemental_core::auth::{authorizer::Authorizer, authorizers::offline::OfflineAuthorizer};
use elemental_launcher::{DriverSpec, LaunchOptions, Launcher};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let launcher = Launcher::builder()
        .storage_root(".minecraft")
        .build();

    let prepared = launcher
        .prepare(
            "Vanilla-1.20.1".to_owned(),
            DriverSpec::vanilla("1.20.1".into()),
        )
        .await?;

    let credential = OfflineAuthorizer {
        username: "Player".to_owned(),
    }
    .authorize()
    .await?;

    let mut command = launcher
        .build_launch_command(
            &prepared,
            &credential,
            &LaunchOptions::default(),
        )
        .await?;
    command.add_jvm_argument("-Xmx4G".to_owned());

    println!(
        "{} with {}",
        prepared.driver().id,
        command.program().display()
    );
    Ok(())
}
```

`prepare()` downloads and writes missing artifacts with a default download
session. To start the game, pass the command to `launch()` or
`launch_logged()`.

Authentication is independent from launch. An `Authorizer` returns a
`UserCredential`; `build_launch_command()` injects that credential into the
game arguments and returns a mutable `LaunchCommand`. Authentication-specific
JVM arguments can be appended after that without changing the credential type.

### Observing preparation progress

`begin_prepare()` keeps the session lifecycle inside the launcher while
exposing the session to the host. The session state derives a structured
progress view and publishes it on a watch channel:

```rust
let preparation = launcher
    .begin_prepare(
        "Vanilla-1.20.1".to_owned(),
        DriverSpec::vanilla("1.20.1".into()),
    )
    .await?;
let progress = preparation.session().progress_receiver();

let prepared = preparation.prepare().await?;
println!("{:?}", progress.borrow().active_tasks);
```

The view exposes active tasks, the last-seen group, and task and byte counters.
Formatting task and group names is left to the UI.

### Custom download sessions

Use `prepare_with_session()` when the host needs custom retry or rate-limit
settings but still wants the launcher to finish and validate the session:

```rust
let session = launcher
    .downloader()
    .session("prepare Vanilla-1.20.1")
    .parallelism(8)
    .open()
    .await?;

let prepared = launcher
    .prepare_with_session(
        session,
        "Vanilla-1.20.1".to_owned(),
        DriverSpec::vanilla("1.20.1".into()),
    )
    .await?;
```

Use `prepare_in_session()` when the caller also needs to own the session
lifecycle, cancellation, or the download report:

```rust
let session = launcher
    .downloader()
    .session("prepare Vanilla-1.20.1")
    .parallelism(8)
    .open()
    .await?;

let prepared = launcher
    .prepare_in_session(
        &session,
        "Vanilla-1.20.1".to_owned(),
        DriverSpec::vanilla("1.20.1".into()),
    )
    .await?;

let report = session.finish().await?;
report.success()?;
```

The session exposes `progress_receiver()`, `subscribe()`, and `snapshot()` for
UI progress reporting.

### Auth-specific JVM arguments

```rust
let mut command = launcher
    .build_launch_command(&prepared, &credential, &LaunchOptions::default())
    .await?;
command.add_jvm_argument(format!(
    "-javaagent:{}",
    authlib_injector.display()
));
let game = launcher.launch(command)?;
```

## Instance Lifecycle

| Operation | Method |
| --- | --- |
| Discover local instances | `inspect_instances()` |
| Detect one instance and its driver | `inspect_instance(name)` |
| Install or refresh an instance | `prepare(name, driver)` |
| Prepare with an observed session | `begin_prepare(name, driver).prepare()` |
| Install with a custom session and managed lifecycle | `prepare_with_session(session, name, driver)` |
| Install with a custom session and caller-owned lifecycle | `prepare_in_session(&session, name, driver)` |
| Reload an existing installation | `load_instance(instance)` |
| Build a command without spawning | `build_launch_command(...)` |
| Spawn a customized command | `launch(command)` / `launch_logged(command)` |
| Build and spawn in one call | `launch_prepared_instance(...)` |
| Launch an installed instance by name | `launch_instance(...)` |

Catalogs are not wrapped by the launcher. Use
`DriverCatalog::for_driver(driver)?.game_versions()` and
`DriverCatalog::for_driver(driver)?.loader_versions(&game_version)` directly.

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
> in launchpad, or inspect before loading, when network access is unavailable.

## Driver Specifications

`DriverSpec` selects the installation flow:

| Variant | Loader version |
| --- | --- |
| `DriverSpec::vanilla(game_version)` | Game version only |
| `FABRIC_DRIVER`, `LEGACY_FABRIC_DRIVER`, `BABRIC_DRIVER`, `QUILT_DRIVER` | Game and loader version |
| `LITELOADER_DRIVER`, `RIFT_DRIVER` | Game and loader version |
| `FORGE_DRIVER`, `NEOFORGE_DRIVER`, `CLEANROOM_DRIVER`, `OUTLANDS_DRIVER` | Game and loader version |

Vanilla has no loader version and uses its own constructor. `DriverSpec::any()`
is for loader-driven distributions and requires all three values:

```rust
use elemental_driver::descriptors::FABRIC_DRIVER;

let driver = DriverSpec::any(
    FABRIC_DRIVER,
    "1.20.1".into(),
    "0.16.0".into(),
)?;
```

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

### Origin Overrides

`origin_overrides()` receives one configuration block per driver, so each
driver keeps its own typed origin enum instead of collapsing them into a
launcher-wide list. Overrides replace the base URL while preserving the
upstream path suffix:

```rust
use elemental_driver::drivers::{
    fabric::source::FabricOrigin,
    vanilla::source::VanillaOrigin,
};
use elemental_launcher::Launcher;

let launcher = Launcher::builder()
    .origin_overrides(|origins| {
        origins.vanilla().override_origin(
            VanillaOrigin::Resources,
            "https://mirror.example/assets".to_owned(),
        )?;
        origins.fabric().override_origin(
            FabricOrigin::Maven,
            "https://mirror.example/fabric-maven".to_owned(),
        )?;
        Ok(())
    })?
    .build();
```

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
- [launchpad](https://github.com/ElectronicWave/Elemental/blob/main/crates/launchpad/README.md)
  shows the complete CLI flow.
