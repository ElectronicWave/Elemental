## Elemental

[![Codacy Badge](https://app.codacy.com/project/badge/Grade/893a020791bf486a9ef80f67729dc2f4)](https://app.codacy.com/gh/ElectronicWave/Elemental/dashboard?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ElectronicWave/Elemental)

[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![License][license-shield]][license-url]

Elemental is a Modern Minecraft Launcher SDK⚛

## Support Matrix

| Family / Driver | Catalog | Inspect | Install | Load Installed | Launch |
| --------------- | ------- | ------- | ------- | -------------- | ------ |
| Vanilla         | ✅       | ✅       | ✅       | ✅              | ✅      |
| Fabric-like     | ✅       | ✅       | ✅       | ✅              | ✅      |
| Fabric          | ✅       | ✅       | ✅       | ✅              | ✅      |
| LegacyFabric    | ✅       | ✅       | ✅       | ✅              | ✅      |
| Babric          | ✅       | ✅       | ✅       | ✅              | ✅      |
| Quilt           | ✅       | ✅       | ✅       | ✅              | ✅      |
| Forge           | ✅       | ✅       | ✅       | ✅              | ✅      |
| NeoForge        | ✅       | ✅       | ✅       | ✅              | ✅      |
| CleanroomMC     | ✅       | ✅       | ✅       | ✅              | ✅      |
| Rift            | ✅       | ✅       | ✅       | ✅              | ✅      |
| LiteLoader      | ✅       | ✅       | ✅       | ✅              | ✅      |

Development is actively in progress. The matrix reflects the current workspace state rather than a stability guarantee. Verified anchors and current range claims live in [ROADMAP.md](ROADMAP.md).

## Why Elemental

- It treats Minecraft distributions as real families instead of flattening everything into one fake `version.json` model. Vanilla, Fabric-like loaders, direct profiled legacy-era loaders such as Rift and LiteLoader, and installer-driven ecosystems such as Forge, NeoForge, and Cleanroom all land on intentional substrates instead of collapsing into special-case flags.
- `Storage` + `Layout` make paths a typed capability, not stringly-typed launcher glue. Game roots, instances, libraries, assets, and version artifacts are resolved through explicit resource models, which keeps alternative layouts and migration work possible without hardcoding one `.minecraft` shape.
- The instance lifecycle is explicit and product-friendly. Elemental separates catalog, inspect, install, load-installed, and launch so a launcher can discover local state, reopen prepared instances, and build launch commands without rerunning the whole install path every time.
- Runtime handling is part of the kernel, not scattered around app code. The core can validate an explicit Java executable against the required major version or resolve a compatible local runtime automatically, which matters once old and new distributions coexist in the same launcher.
- Older tweaker-era loaders do not automatically force a new kernel family. Rift and LiteLoader now run on the direct profiled `version_json` path, which keeps the boot surface smaller until a future target proves otherwise.
- Installer-driven ecosystems are first-class. Forge, NeoForge, and now Cleanroom run on a shared installer-family flow with family-specific merge and runtime behavior, so the SDK can support legacy-derived installers without pretending they are just metadata overlays.
- The workspace is split for reuse instead of forcing one monolith:
  - `schema` for protocol types
  - `core` for storage, runtime, and launch primitives
  - `infra` for downloading and archive work
  - `driver` for catalogs, families, and distribution logic
  - `shared` for persisted state helpers

## Workspace Layout

- `crates/schema`: Pure protocol and serialization types
- `crates/core`: Launcher domain logic, storage, runtime lookup, and launch primitives
- `crates/infra`: Downloader and execution reports
- `crates/driver`: Distribution and driver-specific logic
- `crates/object`: Shared typed object pool
- `crates/shared`: Versioned persisted loader, profile, and store utilities
- `crates/elemental`: Re-export facade crate
- `crates/demo`: End-to-end example

## Quick Start

If you just want to verify the current end-to-end example inside this repository:

```bash
cargo run -p demo
```

The current default demo prepares and launches a Fabric instance.

Loader-specific demo entry points are also available, including `cargo run -p demo -- cleanroom --help`, `cargo run -p demo -- rift --help`, and `cargo run -p demo -- liteloader --help`.

The default demo settings live in [crates/demo/src/main.rs](crates/demo/src/main.rs).

## Use The Library Crates Directly

This is the smallest end-to-end flow using the library crates directly.

### Dependencies

```toml
[dependencies]
anyhow = "1"
tokio = { version = "1", features = ["macros", "process", "rt-multi-thread"] }
elemental = { package = "elemental-kit", version = "0.1" }
```

### 1. Fetch Catalog Data

```rust
use anyhow::Result;
use elemental::{driver::drivers::vanilla::catalog::VanillaCatalog, launcher::Launcher};

#[tokio::main]
async fn main() -> Result<()> {
    let launcher = Launcher::builder().build();
    let data = launcher.catalog(VanillaCatalog::with_defaults()).await?;
    println!("Data: {:#?}", data);

    Ok(())
}
```

## 2. Prepare and Launch

If you want to launch a version that is already fully prepared on disk without downloading anything,
load it from storage first and then launch it.

```rust
use anyhow::Result;
use elemental::{
    core::auth::authorizers::offline::OfflineAuthorizer,
    launcher::{DriverSpec, LaunchOptions, Launcher, PrepareInstanceRequest, VanillaSpec},
};

// Extracting native should use multi-thread runtime to avoid blocking the async flow, but the rest of the work is single-thread-friendly so the default runtime is fine for most of the flow.
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let launcher = Launcher::builder().build();
    let prepared = launcher
        .prepare_instance(PrepareInstanceRequest {
            instance_name: "Vanilla".to_string(),
            driver: DriverSpec::Vanilla(VanillaSpec {
                game_version: "1.20.1".into(),
            }),
        })
        .await?;

    let mut instance = launcher
        .launch_prepared_instance(
            &prepared,
            OfflineAuthorizer {
                username: "Vanilla123".to_string(),
            },
            &LaunchOptions::default(),
        )
        .await?;
    let exit_code = instance.child.wait().await?;
    println!("Exited with code: {exit_code}");
    Ok(())
}
```

## 3. Inspect, Load, and Launch

```rust
use anyhow::Result;
use elemental::{
    core::auth::authorizers::offline::OfflineAuthorizer,
    launcher::{LaunchOptions, Launcher},
};

// Extracting native should use multi-thread runtime to avoid blocking the async flow, but the rest of the work is single-thread-friendly so the default runtime is fine for most of the flow.
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let launcher = Launcher::builder().build();
    // Find all instances in the storage and print them out
    let instances = launcher.inspect_instances().await?;
    println!("Instances: {:#?}", instances);
    let instance = launcher
        .inspect_instance("MyNeoForge-1.21.1".into())
        .await?;
    println!("Instance: {:#?}", instance);
    if let Some(instance) = instance {
        let prepared = launcher.load_instance(instance).await?;
        let mut instance = launcher
            .launch_prepared_instance(
                &prepared,
                OfflineAuthorizer {
                    username: "Fox".into(),
                },
                &LaunchOptions::default(),
            )
            .await?;
        let exit = instance.child.wait().await?;
        println!("Instance exited with: {exit}");
    }
    Ok(())
}
```

## Notes

- This example assumes you already have a compatible local Java runtime.
- Elemental now auto-selects a local runtime using the Minecraft version metadata.
- Runtime discovery uses sources such as the Windows registry, `PATH`, package-manager locations, and `JAVA_HOME`.
- The example uses offline auth on purpose so the minimal flow stays easy to run.
- If you want a complete runnable reference from this repository, [crates/demo/src/main.rs](crates/demo/src/main.rs) is the best starting point.

## Credits

- [MultiMC](https://multimc.org/)
- [Prism Launcher](https://prismlauncher.org/)
- [XMCL](https://xmcl.app/)

<!-- LINKS -->
[contributors-shield]: https://img.shields.io/github/contributors/ElectronicWave/Elemental.svg?style=for-the-badge
[contributors-url]: https://github.com/ElectronicWave/Elemental/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/ElectronicWave/Elemental.svg?style=for-the-badge
[forks-url]: https://github.com/ElectronicWave/Elemental/network/members
[stars-shield]: https://img.shields.io/github/stars/ElectronicWave/Elemental.svg?style=for-the-badge
[stars-url]: https://github.com/ElectronicWave/Elemental/stargazers
[issues-shield]: https://img.shields.io/github/issues/ElectronicWave/Elemental.svg?style=for-the-badge
[issues-url]: https://github.com/ElectronicWave/Elemental/issues
[license-shield]: https://img.shields.io/github/license/ElectronicWave/Elemental.svg?style=for-the-badge
[license-url]: https://github.com/ElectronicWave/Elemental/blob/master/LICENSE
