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
| LiteLoader      | ❌       | ❌       | ❌       | ❌              | ❌      |

Development is actively in progress. The matrix reflects the current workspace state rather than a stability guarantee. Verified anchors and current range claims live in [ROADMAP.md](ROADMAP.md).

## Why Elemental

- It treats Minecraft distributions as real families instead of flattening everything into one fake `version.json` model. Vanilla, Fabric-like loaders, Forge, NeoForge, and Cleanroom already land on different substrates without collapsing into special-case flags.
- `Storage` + `Layout` make paths a typed capability, not stringly-typed launcher glue. Game roots, instances, libraries, assets, and version artifacts are resolved through explicit resource models, which keeps alternative layouts and migration work possible without hardcoding one `.minecraft` shape.
- The instance lifecycle is explicit and product-friendly. Elemental separates catalog, inspect, install, load-installed, and launch so a launcher can discover local state, reopen prepared instances, and build launch commands without rerunning the whole install path every time.
- Runtime handling is part of the kernel, not scattered around app code. The core can validate an explicit Java executable against the required major version or resolve a compatible local runtime automatically, which matters once old and new distributions coexist in the same launcher.
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

The current demo prepares and launches a Fabric instance.

Loader-specific demo entry points are also available, including `cargo run -p demo -- cleanroom --help`.

The default demo settings live in [crates/demo/src/main.rs](crates/demo/src/main.rs).

## Vanilla Download And Launch Example

This is the smallest end-to-end flow using the library crates directly.

### Dependencies

```toml
[dependencies]
anyhow = "1"
tokio = { version = "1", features = ["macros", "process", "rt-multi-thread"] }
elemental = { path = "crates/elemental" }
```

### Example

```rust
use std::path::PathBuf;

use anyhow::Result;
use elemental::{
    core::{auth::authorizers::offline::OfflineAuthorizer, storage::Storage},
    driver::drivers::{
        vanilla::{config::VanillaLaunchConfig, driver::VanillaDriver},
        version_json::{BaseLayout, VersionJsonGameStorageExt},
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    let storage = Storage::new(PathBuf::from(".minecraft"), BaseLayout);
    let instance = storage.instance("MyGame-1.16.5".to_owned(), BaseLayout)?;
    let vanilla = VanillaDriver::with_defaults()?;
    let launch_config = VanillaLaunchConfig::new();
    let authorizer = OfflineAuthorizer {
        username: "Player".to_owned(),
    };

    let prepared = vanilla.prepare(&instance, "1.16.5".to_owned()).await?;
    let launched = vanilla.launch(prepared, &launch_config, authorizer).await?;
    println!("java executable: {}", launched.runtime.executable().display());
    println!(
        "install status: {:?}",
        launched.prepared_version.install_status
    );

    let mut child = launched.child;
    let exit_status = child.wait().await?;
    println!("game exited with: {exit_status}");

    Ok(())
}
```

## Launch An Existing Local Version

If you want to launch a version that is already fully prepared on disk without downloading anything,
load it from storage first and then launch it.

```rust
use std::path::PathBuf;

use anyhow::Result;
use elemental::{
    core::{auth::authorizers::offline::OfflineAuthorizer, storage::Storage},
    driver::drivers::{
        vanilla::{config::VanillaLaunchConfig, driver::VanillaDriver},
        version_json::{BaseLayout, VersionJsonGameStorageExt},
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    let storage = Storage::new(PathBuf::from(".minecraft"), BaseLayout);
    let instance = storage.instance("MyGame-1.16.5".to_owned(), BaseLayout)?;
    let vanilla = VanillaDriver::with_defaults()?;
    let launch_config = VanillaLaunchConfig::new();
    let authorizer = OfflineAuthorizer {
        username: "Player".to_owned(),
    };

    let prepared = vanilla.load_prepared(&instance)?;
    let launched = vanilla.launch(prepared, &launch_config, authorizer).await?;
    let mut child = launched.child;
    let exit_status = child.wait().await?;
    println!("game exited with: {exit_status}");

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
