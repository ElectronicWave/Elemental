## Elemental
[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![License][license-shield]][license-url]
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/893a020791bf486a9ef80f67729dc2f4)](https://app.codacy.com/gh/ElectronicWave/Elemental/dashboard?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ElectronicWave/Elemental)

**Elemental is a Rust workspace for building Minecraft launchers.**

It currently provides the pieces needed for a practical vanilla-launch workflow:

- Mojang protocol schemas in `elemental-schema`
- version resolution and launch metadata handling in `elemental-core`
- artifact downloading in `elemental-infra`
- Java runtime discovery from local providers
- a high-level vanilla launcher flow that resolves, readies, and launches an instance
- an executable demo that downloads a version and launches it offline

## Workspace Layout

- `crates/schema`: Pure protocol and serialization types
- `crates/core`: Launcher domain logic, Mojang services, storage, runtime lookup, launch builder
- `crates/infra`: Downloader and execution reports
- `crates/loader`: Mod-loader metadata integrations
- `crates/object`: Shared typed object pool
- `crates/shared`: Versioned persisted config/profile utilities
- `crates/elemental`: Re-export facade crate
- `crates/demo`: End-to-end example

## Quick Start

If you just want to verify the current example inside this repository:

```bash
cargo run -p demo
```

The demo will:

1. Resolve Mojang metadata for a vanilla version
2. Ready the local instance, downloading missing jars, libraries, assets, and logging config
3. Extract native libraries when needed
4. Auto-select a compatible local Java runtime from the version metadata
5. Launch the game with an offline account

The default demo settings live in [crates/demo/src/lib.rs](crates/demo/src/lib.rs).

## Vanilla Download And Launch Example

This is the smallest end-to-end flow using the library crates directly.

### Dependencies

```toml
[dependencies]
anyhow = "1"
tokio = { version = "1", features = ["macros", "process", "rt-multi-thread"] }
elemental-core = { path = "crates/core" }
```

### Example

```rust
use std::path::PathBuf;

use anyhow::Result;
use elemental_core::{
    auth::authorizers::offline::OfflineAuthorizer,
    launcher::vanilla::{VanillaLaunchOptions, VanillaLauncher, VanillaVersionSpec},
    storage::{game::GameStorage, layout::BaseLayout},
};

#[tokio::main]
async fn main() -> Result<()> {
    let storage = GameStorage::new(PathBuf::from(".minecraft"), BaseLayout);
    let launcher = VanillaLauncher::with_defaults()?;
    let launch_options = VanillaLaunchOptions::new(VanillaVersionSpec::new(
        "1.16.5".to_owned(),
        "MyGame-1.16.5".to_owned(),
        BaseLayout,
    ));
    let authorizer = OfflineAuthorizer {
        username: "Player".to_owned(),
    };

    let launched = launcher.launch(&storage, &launch_options, authorizer).await?;
    println!("java executable: {}", launched.runtime.executable().display());
    println!("install status: {:?}", launched.ready_version.install_status);

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
- If you want a complete runnable reference from this repository, [crates/demo/src/lib.rs](crates/demo/src/lib.rs) is the best starting point.

## Wiki

- DeepWiki: <https://deepwiki.com/ElectronicWave/Elemental>
- Codacy dashboard: <https://app.codacy.com/gh/ElectronicWave/Elemental/dashboard>

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
