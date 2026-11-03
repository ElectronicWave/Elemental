# elemental-driver

Catalog, inspect, prepare, and launch Minecraft distributions across vanilla and
modloader ecosystems.

`elemental-driver` turns upstream metadata into one normalized installation
model. It fetches Mojang, Fabric, Quilt, and Forge-family manifests, resolves
the files and arguments required by each distribution, and exposes
loader-specific catalogs and drivers. The crate is independent of the
high-level instance lifecycle, so its catalogs and driver families can also be
used directly.

## Features

- Catalogs for Vanilla, Fabric, Legacy Fabric, Babric, Quilt, LiteLoader, Rift,
  Forge, NeoForge, and Cleanroom.
- One `Catalog` interface that groups releases by `MinecraftVersionId`.
- Direct, profiled, and installer-driven installation families.
- Local instance inspection with driver and loader-version detection.
- Configurable origins and mirror overrides for restricted networks.
- Typed Java runtime, resolution, quick-play, and extra-argument launch options.
- Shared version JSON layouts for game roots and individual instances.

## Installation

```toml
[dependencies]
anyhow = "1"
elemental-driver = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

List the Minecraft versions known to the Vanilla catalog:

```rust
use elemental_driver::{
    catalog::Catalog,
    drivers::vanilla::catalog::VanillaCatalog,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let releases = VanillaCatalog::with_defaults().releases().await?;
    println!("{} game versions", releases.len());
    Ok(())
}
```

Loader catalogs use the same interface. Fabric exposes an additional flavor
selector for Legacy Fabric and Babric:

```rust
use elemental_driver::{
    catalog::Catalog,
    drivers::fabric::{
        catalog::FabricCatalog,
        source::FabricFlavor,
    },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let releases = FabricCatalog::for_flavor(FabricFlavor::Fabric)?
        .releases()
        .await?;
    println!("{} Fabric game versions", releases.len());
    Ok(())
}
```

Forge-family catalogs use their `Default` implementation:

```rust
use elemental_driver::{
    catalog::Catalog,
    drivers::{
        cleanroom::catalog::CleanroomCatalog,
        forge::catalog::ForgeCatalog,
        neoforge::catalog::NeoForgeCatalog,
    },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let forge = ForgeCatalog::default().releases().await?;
    let neoforge = NeoForgeCatalog::default().releases().await?;
    let cleanroom = CleanroomCatalog::default().releases().await?;

    println!(
        "{} Forge, {} NeoForge, {} Cleanroom versions",
        forge.len(),
        neoforge.len(),
        cleanroom.len()
    );
    Ok(())
}
```

## Driver Families

| Family | Drivers | Installation model |
| --- | --- | --- |
| Direct version JSON | Vanilla | Resolve Mojang metadata and install the client, libraries, and assets directly |
| Fabric profiles | Fabric, Legacy Fabric, Babric, Quilt | Merge a loader profile with the base Vanilla version |
| Legacy profiles | Rift, LiteLoader | Merge a legacy profiled distribution with its required libraries |
| Installer-driven | Forge, NeoForge, Cleanroom | Download and run the installer, then apply its profile and processors |

Every driver identifies an installed instance through the `Driver` trait. The
family-specific drivers add preparation, loading, and launch-command
construction.

## Modules

| Module | Contents |
| --- | --- |
| `catalog` | `Catalog` plus helpers for grouping releases by game version |
| `descriptors` | Stable driver identifiers and display names |
| `driver` | `DriverDescriptor`, `InstalledDriver`, and the `Driver` trait |
| `drivers` | Per-loader catalogs, sources, drivers, configs, and prepared state |
| `families` | Shared version JSON and installer-driven implementation families |
| `inspect` | Instance probes, installed-driver detection, and library markers |
| `loader_version` | The typed `LoaderVersionId` |
| `url` | `Origin` and `OriginPolicy` primitives |

## Core Concepts

### Catalogs

`Catalog::releases()` returns a map keyed by game version. Release records
contain the loader version and any useful upstream description, such as a
Vanilla release type or Fabric stability flag.

> [!NOTE]
> NeoForge game versions are inferred from the loader version naming. The
> installer profile remains the authoritative source when preparing an
> instance.

### Origins and mirrors

Every remote family declares its known origins. `OriginPolicy` rewrites upstream
URLs to configured overrides without changing the rest of the metadata.

- `VanillaEndpoints::mirror(...)` overrides all five Mojang origins at once.
- Fabric-family endpoints support `for_flavor(...)` and explicit overrides.
- Installer-driven sources expose Maven and installer endpoints.

This keeps mirror support in the source layer instead of spreading URL logic
through drivers.

### Inspection

`InstanceProbe` reads local version metadata. `InstalledInstance::detect()` and
`detect_all()` test available drivers and return the matching descriptor,
loader version, and game version. The high-level launcher uses the same
inspection path when reopening an existing instance.

### Launch configuration

`VersionJsonLaunchConfig` is shared by all version JSON drivers. It controls:

- Runtime selection, an explicit executable, and strict or disabled validation.
- Launcher name and version, client ID, and game resolution.
- Quick-play targets.
- Extra JVM and game arguments.

## Related Crates

- [elemental-core](https://github.com/ElectronicWave/Elemental/blob/main/crates/core/README.md)
  provides storage, runtime, authentication, and process primitives.
- [elemental-infra](https://github.com/ElectronicWave/Elemental/blob/main/crates/infra/README.md)
  provides the downloader used during installation.
- [elemental-schema](https://github.com/ElectronicWave/Elemental/blob/main/crates/schema/README.md)
  defines the upstream manifest models.
- [elemental-launcher](https://github.com/ElectronicWave/Elemental/blob/main/crates/launcher/README.md)
  exposes the high-level instance lifecycle built on these drivers.
- [elemental-kit](https://github.com/ElectronicWave/Elemental/blob/main/README.md#the-elemental-kit-facade)
  re-exports this crate behind the `driver` feature.
