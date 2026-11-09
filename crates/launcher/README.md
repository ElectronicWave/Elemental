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
- Injected runtime providers with a per-launcher distribution cache.
- Launch-command construction for hosts that manage process creation.
- Process spawning through the core launch primitives.
- Typed runtime and launch options for runtime selection, resolution, quick
  play, and extra arguments.
- A stable read-only view of prepared instance metadata, natives, and the
  resolved classpath.

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
use elemental_launcher::{DriverSpec, LaunchOptions, Launcher, RuntimeOptions};

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

    let credential = OfflineAuthorizer {
        username: "Player".to_owned(),
    }
    .authorize()
    .await?;

    let mut command = launcher
        .build_launch_command(
            &prepared,
            &credential,
            &RuntimeOptions::default(),
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
session. To start the game, pass the command to `launch()`.

Inspect a prepared instance without touching driver internals:

```rust
let info = prepared.info()?;
println!(
    "{} requires Java {} and has {} classpath entries",
    info.metadata.id,
    info.metadata.required_java_major_version,
    info.classpath.len()
);
```

Authentication is independent from launch. An `Authorizer` returns a
`UserCredential`; `build_launch_command()` injects that credential into the
game arguments and returns a mutable `LaunchCommand`. Authentication-specific
JVM arguments can be appended after that without changing the credential type.

### Observing preparation progress

Create and observe the session before passing it to `prepare_with_session()`.
The launcher finishes and validates the session after preparation:

```rust
let session = launcher
    .downloader()
    .session("prepare Vanilla-1.20.1")
    .open()
    .await?;
let progress = session.progress_receiver();

let prepared = launcher
    .prepare_with_session(
        session,
        "Vanilla-1.20.1".to_owned(),
        DriverSpec::vanilla("1.20.1".into()),
        &RuntimeOptions::default(),
    )
    .await?;
println!("{:?}", progress.borrow().active_tasks);
```

The view exposes active tasks, the last-seen group, and task and byte counters.
Formatting task and group names is left to the UI.

The session is the only progress source. Use `progress_receiver()` for
snapshots or `subscribe()` for structured download events; the launcher does not
wrap either stream.

### Custom download sessions

Use `prepare_with_session()` when the host needs custom retry or rate-limit
settings. Build and observe the session first, then pass ownership to the
launcher for finishing and validation:

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
        &RuntimeOptions::default(),
    )
    .await?;
```

The session exposes `progress_receiver()`, `subscribe()`, and `snapshot()` for
UI progress reporting.

### Auth-specific JVM arguments

```rust
let mut command = launcher
    .build_launch_command(
        &prepared,
        &credential,
        &RuntimeOptions::default(),
        &LaunchOptions::default(),
    )
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
| Install or refresh an instance | `prepare(name, driver, runtime)` |
| Install with an observed or configured session | `prepare_with_session(session, name, driver, runtime)` |
| Reload an existing installation | `load_instance(instance)` |
| Build a command without spawning | `build_launch_command(...)` |
| Spawn a customized command | `launch(command)` |
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

When the driver comes from inspection or a CLI, build the spec from the
descriptor instead of matching on it:

```rust
let game_version = instance
    .driver
    .game_version
    .clone()
    .ok_or_else(|| anyhow::anyhow!("instance has no game version"))?;
let spec = DriverSpec::from_descriptor(
    instance.driver.driver,
    game_version,
    loader_version,
)?;
```

`from_descriptor()` rejects a loader version for Vanilla and requires one for
every loader-driven distribution.

## Launch Options

Launch configuration is split by lifecycle:

- `RuntimeOptions` covers `runtime_major_version`, `runtime_override`, and
  `runtime_validation`.
- `LaunchOptions` covers launcher name and version, client ID, window
  resolution, quick-play targets, and extra JVM and game arguments.

`LaunchOptions` is owned by `elemental-launcher`. The driver crate only exposes
`RuntimeOptions`; version JSON launch configuration stays an implementation
detail behind `Launcher::build_launch_command()`.

Use `RuntimeOptions::default()` for automatic runtime selection with strict
Java version validation, and `LaunchOptions::default()` when no branding or
window customization is needed.

Pass the same `RuntimeOptions` to preparation and launch. Installer-driven
families resolve their client-processor runtime during preparation, so the
override must be available there as well.

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

Use `root_layout()` and `instance_layout()` for custom layouts. All launcher
construction goes through `Launcher::builder()`.

### Custom layouts

A layout only maps typed resources to paths; `storage_root` remains the game
root. Implement `Layout` with `Resource = VersionJsonRootResource` or
`Resource = VersionJsonInstanceResource`; the marker traits are implemented
automatically. Delegating unknown resources to the base layout keeps the
custom layout complete:

```rust
use std::path::{Path, PathBuf};

use elemental_core::storage::layout::Layout;
use elemental_driver::families::version_json::{
    BaseInstanceLayout, BaseRootLayout, VersionJsonInstanceResource, VersionJsonRootResource,
};
use elemental_launcher::Launcher;

#[derive(Debug, Clone, Default)]
struct SharedLibrariesRoot;

impl Layout for SharedLibrariesRoot {
    type Resource = VersionJsonRootResource;
    type ExtendedResource = VersionJsonRootResource;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf> {
        match resource {
            VersionJsonRootResource::Libraries(relative) => {
                let libraries_root = root.join("shared").join("libraries");
                Some(match relative {
                    Some(relative) => libraries_root.join(relative.as_path()),
                    None => libraries_root,
                })
            }
            other => BaseRootLayout.get_resource(root, other),
        }
    }

    fn get_extended_resource(
        &self,
        root: &Path,
        resource: Self::ExtendedResource,
    ) -> Option<PathBuf> {
        self.get_resource(root, resource)
    }

    fn name(&self) -> &'static str {
        "SharedLibrariesRoot"
    }
}

#[derive(Debug, Clone, Default)]
struct FlatInstanceLayout;

impl Layout for ExtendedInstanceLayout {
    type Resource = VersionJsonInstanceResource;
    type ExtendedResource = VersionJsonInstanceResource;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf> {
        match resource {
            VersionJsonInstanceResource::Jar => Some(root.join("client.jar")),
            other => BaseInstanceLayout.get_resource(root, other),
        }
    }

    fn get_extended_resource(
        &self,
        root: &Path,
        resource: Self::ExtendedResource,
    ) -> Option<PathBuf> {
        self.get_resource(root, resource)
    }

    fn name(&self) -> &'static str {
        "FlatInstanceLayout"
    }
}

let launcher = Launcher::builder()
    .storage_root(".minecraft")
    .root_layout(SharedLibrariesRoot)
    .instance_layout(FlatInstanceLayout)
    .build();
```

Root layouts receive the game root; instance layouts receive the instance
directory. Custom instance layouts must be `Clone + Send`, and both layouts
must be `Clone` because the launcher clones them into scoped storage views.

### Extending resources

`VersionJsonRootResource` and `VersionJsonInstanceResource` are closed enums:
the launcher and drivers match them to locate metadata, libraries, assets, and
natives. Hosts cannot add variants to them.

Use `Layout::ExtendedResource` for resources that only your application
consumes. Wrap the standard resource in your own enum and delegate through
`get_resource()`:

```rust
#[derive(Debug, Clone, Default)]
struct ExtendedInstanceLayout;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CustomInstanceResource {
    Standard(VersionJsonInstanceResource),
    Screenshots,
}

impl Layout for FlatInstanceLayout {
    type Resource = VersionJsonInstanceResource;
    type ExtendedResource = CustomInstanceResource;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf> {
        match resource {
            VersionJsonInstanceResource::Jar => Some(root.join("client.jar")),
            other => BaseInstanceLayout.get_resource(root, other),
        }
    }

    fn get_extended_resource(
        &self,
        root: &Path,
        resource: Self::ExtendedResource,
    ) -> Option<PathBuf> {
        match resource {
            CustomInstanceResource::Standard(resource) => self.get_resource(root, resource),
            CustomInstanceResource::Screenshots => Some(root.join("screenshots")),
        }
    }

    fn name(&self) -> &'static str {
        "ExtendedInstanceLayout"
    }
}
```

When the launcher uses this layout, read extended resources through
`Layoutable`:

```rust
use elemental_core::storage::layout::Layoutable;

let screenshots = prepared
    .version()
    .try_get_extended_resource(CustomInstanceResource::Screenshots)?;
```

If a resource must participate in installation or launch, it cannot be added
from the host; it belongs in the driver's resource enum and its installation
or launch logic.

Runtime discovery is a launcher dependency rather than a process-wide setting.
Pass providers explicitly when the host needs its own set:

```rust
use elemental_core::runtime::provider::default_providers;

let launcher = Launcher::builder()
    .runtime_providers(default_providers())
    .build();

for runtime in launcher.runtime_resolver().distributions().await {
    println!("{}", runtime.executable().display());
}
```

The launcher caches distributions per `RuntimeResolver`, so multiple launchers
can use different provider lists in the same process without sharing state.

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
