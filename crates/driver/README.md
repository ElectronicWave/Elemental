# elemental-driver

Catalog, inspect, prepare, and launch Minecraft distributions across vanilla and
modloader ecosystems.

`elemental-driver` turns upstream metadata into one normalized installation
model. It fetches Mojang, Fabric, Quilt, Forge-family, and legacy loader
manifests, resolves the files and arguments required by each distribution, and
exposes typed catalogs and drivers. The crate is independent of the high-level
instance lifecycle, so its catalogs and driver families can also be used
directly.

> [!NOTE]
> Driver selection is type-driven. Descriptors are stable identifiers used for
> display and persistence; they are not used to decide which implementation to
> run.

## Features

- Catalogs for Vanilla, Fabric, Legacy Fabric, Babric, Quilt, LiteLoader, Rift,
  Forge, NeoForge, Cleanroom, and Outlands.
- Separate `GameVersionCatalog` and `LoaderVersionCatalog` contracts.
- Direct, profiled, installer-driven, and RetroFutura installation families.
- Type-safe dispatch through `ProfiledDriver` and `InstallerDriver`.
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

### Game versions

List the Minecraft versions known to the Vanilla catalog:

```rust
use elemental_driver::{
    catalog::GameVersionCatalog,
    drivers::vanilla::catalog::VanillaCatalog,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let versions = VanillaCatalog::with_defaults().game_versions().await?;
    println!("{} game versions", versions.len());
    Ok(())
}
```

### Loader versions

Loader catalogs use `LoaderVersionCatalog`. Fabric exposes a flavor selector for
Legacy Fabric and Babric:

```rust
use elemental_core::minecraft::MinecraftVersionId;
use elemental_driver::{
    catalog::LoaderVersionCatalog,
    drivers::fabric::{
        catalog::FabricCatalog,
        source::{FabricEndpoints, FabricFlavor},
    },
    families::version_json::LoaderMetaSource,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let catalog = FabricCatalog::new(LoaderMetaSource::new(
        FabricEndpoints::for_flavor(FabricFlavor::LegacyFabric)?,
    ));
    let game_version = MinecraftVersionId::from("1.12.2");
    let releases = catalog.loader_versions(&game_version).await?;
    println!("{} Legacy Fabric loader versions", releases.len());
    Ok(())
}
```

Forge-family catalogs use their `Default` endpoint configuration:

```rust
use elemental_core::minecraft::MinecraftVersionId;
use elemental_driver::{
    catalog::LoaderVersionCatalog,
    drivers::{
        cleanroom::CleanroomCatalog,
        forge::ForgeCatalog,
        neoforge::NeoForgeCatalog,
    },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let game_version = MinecraftVersionId::from("1.20.1");
    let forge = ForgeCatalog::with_defaults()
        .loader_versions(&game_version)
        .await?;
    let neoforge = NeoForgeCatalog::with_defaults()
        .loader_versions(&game_version)
        .await?;
    let cleanroom = CleanroomCatalog::with_defaults()
        .loader_versions(&MinecraftVersionId::from("1.12.2"))
        .await?;

    println!(
        "{} Forge, {} NeoForge, {} Cleanroom releases",
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
| Profiled version JSON | Fabric, Legacy Fabric, Babric, Quilt, LiteLoader, Rift | Merge a loader profile with the base Vanilla version |
| Installer-driven | Forge, NeoForge | Download and run a Maven installer, then apply its profile and processors |
| RetroFutura installer | Cleanroom, Outlands | Reuse an old base version while replacing its runtime through an installer |

Every driver implements the `Driver` trait for inspection. The family-specific
drivers add preparation, loading, and launch-command construction.

RetroFutura is the shared path for old Minecraft lines whose runtime is replaced
while the original game metadata remains the base. Cleanroom and Outlands are
two declarations of that model.

## Customization

### Override origins and mirrors

All remote endpoints are typed. Use the endpoint constructors when only the base
URLs change, or implement `Origin` when you need a new endpoint family.

```rust
use elemental_driver::drivers::fabric::{
    driver::FabricDriverFamily,
    source::{FabricEndpointOverrides, FabricFlavor},
};

let driver = FabricDriverFamily::new(FabricFlavor::Fabric)
    .new_driver_with_overrides(FabricEndpointOverrides {
        meta_origin: "https://meta.example.com".to_owned(),
        maven_origin: "https://maven.example.com".to_owned(),
    })?;
```

`build_driver(source)` constructs the driver with the official Vanilla source.
For a complete Vanilla mirror, use `VanillaEndpoints::mirror(...)`, wrap it in
`VanillaSource::new(...)`, then call `.with_vanilla_source(...)`.

### Add a profiled loader

Profiled loaders merge a loader profile with the base Vanilla metadata. Implement
`ProfiledVersionJsonFamily` and provide:

- a `DriverDescriptor`;
- a `Source` that fetches the profile;
- a `RemoteResolver` for loader and Vanilla URLs;
- an optional `ProfiledDriverIdentity` for local-instance detection;
- `profile()` to fetch loader metadata.

The default `merge_profile()` is suitable for Fabric-like pass-through profiles.
Override it when the loader changes libraries or arguments.

```rust
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_driver::{
    driver::DriverDescriptor,
    drivers::vanilla::source::VanillaSource,
    error::Result,
    families::version_json::{
        LoaderMetaSource, ProfiledVersionJsonFamily, ProfiledVersionJsonFamilyExt,
        VanillaFallbackRemoteResolver,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
};
use elemental_schema::fabric::ProfileJson;

const MY_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "myloader",
    name: "MyLoader",
};
const MY_LIBRARIES: LibraryPrefixSet =
    LibraryPrefixSet::new(&["com.example:myloader:"]);
const MY_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    MY_DRIVER,
    MY_LIBRARIES,
    MY_LIBRARIES,
    ProfileIdPattern::new("-myloader-"),
);

// MyEndpoints implements LoaderMetaEndpoints + UpstreamUrlRewriter + Default.
type MySource = LoaderMetaSource<MyEndpoints>;
type MyResolver = VanillaFallbackRemoteResolver<MyEndpoints>;

#[derive(Debug, Clone, Copy, Default)]
struct MyFamily;

#[async_trait(?Send)]
impl ProfiledVersionJsonFamily for MyFamily {
    type Source = MySource;
    type RemoteResolver = MyResolver;

    fn descriptor(&self) -> DriverDescriptor {
        MY_DRIVER
    }

    fn identity(&self) -> Option<ProfiledDriverIdentity> {
        Some(MY_IDENTITY)
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver {
        VanillaFallbackRemoteResolver::new(
            "myloader",
            vanilla_source.endpoints().clone(),
            source.endpoints().clone(),
        )
    }

    async fn profile(
        &self,
        source: &Self::Source,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<ProfileJson> {
        source
            .profile_json(game_version.as_str(), loader_version.as_str())
            .await
    }
}

let driver = MyFamily.build_driver_with_defaults()?;
```

`MyEndpoints` must implement `LoaderMetaEndpoints` and `UpstreamUrlRewriter`.
`build_driver_with_defaults()` also requires `Default`; otherwise construct the
source explicitly and call `build_driver(source).with_vanilla_source(...)`. Use
`ProfileIdPattern::loader_prefixed(...)` when the installed profile id is
`<loader-prefix><version>` instead of `<game>-<loader>-<version>`.

### Add an installer loader

For Maven-published installers, implement `MavenSpec` for the coordinate and
release parsing, then implement `InstallerFamily` with the source, endpoint
types, inspection identity, family name, and loader-version prefix. The usual
aliases are:

```rust
type MyEndpoints = MavenEndpoints<MySpec>;
type MySource = InstallerMavenSource<MyEndpoints>;
type MyDriver = InstallerFamilyDriver<MyFamily>;
type MyCatalog = InstallerMavenCatalog<MyEndpoints>;
```

For a loader that replaces the runtime of an old Minecraft version, implement
`RetroFuturaCatalog` instead and use the shared RetroFutura aliases:

```rust
type MyDriver = RetroFuturaDriver<MyRetroFutura>;
type MyCatalog = RetroFuturaReleaseCatalog<MyRetroFutura>;
```

This is the same declaration used by Cleanroom and Outlands. It keeps base game
metadata, installer state, and launch handling shared instead of duplicating a
legacy-runtime driver.

### Add a fully custom driver

If a loader does not fit any shared family, implement `Driver` directly:

```rust
use async_trait::async_trait;
use elemental_core::storage::layout::Layout;
use elemental_driver::{
    driver::{Driver, DriverDescriptor, InstalledDriver},
    error::Result,
    inspect::InstanceProbe,
};

const MY_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "mydriver",
    name: "MyDriver",
};

struct MyDriver;

#[async_trait]
impl<L: Layout, VL: Layout> Driver<L, VL> for MyDriver {
    fn descriptor(&self) -> DriverDescriptor {
        MY_DRIVER
    }

    async fn inspect(
        &self,
        probe: &InstanceProbe<L, VL>,
    ) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };

        // Match loader-specific metadata and return InstalledDriver::version_json(...).
        Ok(None)
    }
}
```

Custom drivers can be passed directly to `InstalledInstance::detect()` or
`inspect_instances()`. `elemental-launcher` intentionally uses a closed, typed
`DriverSpec`; exposing a custom driver through the high-level launcher requires
adding a `DriverSpec` variant, a descriptor, and a `ResolvedLauncherDriver`
mapping.

## Modules

| Module | Contents |
| --- | --- |
| `catalog` | `GameVersionCatalog`, `LoaderVersionCatalog`, `GameVersion`, `LoaderRelease`, `InstallerMavenCatalog`, and `LoaderMetaCatalog` |
| `descriptors` | Stable driver identifiers and display names |
| `driver` | `DriverDescriptor`, `InstalledDriver`, and the `Driver` trait |
| `drivers` | Per-loader catalogs, sources, drivers, configs, prepared state, and typed dispatch |
| `families` | Shared version JSON, installer, processor, and RetroFutura implementation families |
| `inspect` | Instance probes, installed-driver detection, and library markers |
| `loader_version` | The typed `LoaderVersionId` |
| `url` | `Origin` and `OriginPolicy` primitives |

## Core Concepts

### Catalogs

`GameVersionCatalog::game_versions()` returns the game versions known to a
source. `LoaderVersionCatalog::loader_versions(&game_version)` returns the
loader releases for one game version. `LoaderRelease` carries the typed game and
loader versions plus optional upstream metadata such as stability, stream, and
publication time.

`LoaderMetaCatalog` handles Fabric-style metadata APIs.
`InstallerMavenCatalog` handles installer-driven Maven metadata and uses each
endpoint's `release()` parser.

> [!NOTE]
> NeoForge game versions are inferred from the loader version naming. The
> installer profile remains the authoritative source when preparing an
> instance.

### Origins and mirrors

Every remote family declares typed origins. `OriginPolicy` rewrites upstream
URLs to configured overrides without changing the rest of the metadata.

- `VanillaEndpoints::mirror(...)` overrides all five Mojang origins at once.
- Fabric-family endpoints support `for_flavor(...)` and explicit overrides.
- Installer-driven sources expose Maven and installer endpoints.

This keeps mirror support in the source layer instead of spreading URL logic
through drivers.

### Inspection

`InstanceProbe` reads local version metadata. `InstalledInstance::detect()` and
`detect_all()` test available drivers and return the matching descriptor, loader
version, and game version. `ProfiledDriverIdentity` describes how a family
recognizes its marker libraries, profile ids, and loader versions. The
high-level launcher uses the same inspection path when reopening an existing
instance.

More specific drivers should be inspected before Vanilla, because a modded
installation still contains valid Minecraft metadata.

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
