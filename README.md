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
| NeoForge        | ❌       | ✅       | ❌       | ❌              | ❌      |
| LiteLoader      | ❌       | ❌       | ❌       | ❌              | ❌      |
| CleanroomMC     | ❌       | ❌       | ❌       | ❌              | ❌      |

Development is actively in progress. The matrix reflects the current workspace state rather than a stability guarantee.

The `fabric-like` substrate now backs multiple verified drivers. Modern Fabric, LegacyFabric, Babric, and Quilt all have verified end-to-end smoke coverage on representative anchors.

Forge now has verified installer-family anchors on `1.12.2 / 14.23.5.2860` and `1.20.1 / 47.3.1`. Broader Forge ranges are still not claimed yet.

## Why Elemental

- Built in Rust, so the launcher core gets native performance, predictable memory behavior, and strong typing instead of a large dynamic runtime.
- The cache model is explicit. Artifacts, assets, natives, and instance state are tracked separately, so the launcher can reuse what is valid and only rebuild what is stale.
- `Storage` + `Layout` separate storage semantics from physical paths, which makes migration, compatibility, and custom launcher layouts possible without hardcoding one global directory tree.
- `Driver` is a first-class abstraction. Vanilla, Fabric-like, Forge, and future families are modeled as real distributions instead of hidden special cases layered on top of one default runtime.
- The public flow is instance-first and product-friendly: open or create an instance, install into it, load installed state, then launch.
- `version_json` is treated as a family layer, not as the center of the whole system. That keeps the core open to installer-driven and future non-`version_json` families.
- The workspace is intentionally split by responsibility, so launcher products can reuse only the layers they need:
  - `schema` for protocol types
  - `core` for launcher primitives
  - `infra` for downloading
  - `driver` for distribution logic
  - `shared` for persisted state/config helpers

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
