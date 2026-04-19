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
2. Download the client jar, libraries, assets, and logging config
3. Extract native libraries
4. Discover a local Java runtime
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
elemental-infra = { path = "crates/infra" }
```

### Example

```rust
use std::path::PathBuf;

use anyhow::{Context, Result};
use elemental_core::{
    auth::authorizers::offline::OfflineAuthorizer,
    launcher::builder::LaunchBuilder,
    runtime::{distribution::Distribution, provider::all_providers},
    services::mojang::MojangService,
    storage::{game::GameStorage, layout::BaseLayout},
};
use elemental_infra::downloader::core::ElementalDownloader;

#[tokio::main]
async fn main() -> Result<()> {
    let game_root = PathBuf::from(".minecraft");
    let version_id = "1.16.5".to_owned();
    let version_name = "MyGame-1.16.5".to_owned();

    let storage = GameStorage::new(&game_root, BaseLayout);
    let service = MojangService::default();

    let resolved = service
        .resolve_vanilla_version(&storage, version_id, version_name, BaseLayout)
        .await
        .context("resolve vanilla version failed")?;

    let downloader = ElementalDownloader::with_config_default()
        .context("create downloader failed")?;
    let reports = downloader
        .execute_planner(&resolved.planner())
        .await
        .context("download version artifacts failed")?;
    println!("download reports: {reports:#?}");

    resolved
        .version
        .extract_natives()
        .context("extract natives failed")?;

    let runtime = Distribution::from_providers::<Vec<_>>(all_providers())
        .await
        .into_iter()
        .find(|distribution| {
            distribution
                .release
                .as_ref()
                .and_then(|release| release.jre_version.as_ref())
                .is_some_and(|version| version.starts_with("1.8"))
        })
        .context("can't find a local Java runtime with version prefix 1.8")?;

    let authorizer = OfflineAuthorizer {
        username: "Player".to_owned(),
    };

    let mut child = LaunchBuilder::new(authorizer, runtime, resolved.version)
        .set_username("Player".to_owned())
        .launch()
        .await
        .context("launch game failed")?;

    let exit_status = child.wait().await.context("wait for game failed")?;
    println!("game exited with: {exit_status}");

    Ok(())
}
```

## Notes

- This example assumes you already have a compatible local Java runtime.
- `all_providers()` looks for Java in sources such as the Windows registry, `PATH`, package-manager locations, and `JAVA_HOME`.
- The example uses `OfflineAuthorizer` on purpose so the minimal flow stays easy to run.
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
