# elemental-schema

Serde data models for Minecraft launcher and modloader metadata.

`elemental-schema` is the shared JSON layer used by Elemental. It mirrors the
manifests published by Mojang, Fabric, Quilt, and Forge-family tooling. The
crate performs no network or filesystem access, so it can parse cached
metadata in any Rust application without pulling in the launcher runtime.

## Features

- Mojang launcher version manifests.
- Piston version metadata, arguments, rules, libraries, and asset indexes.
- Fabric and Quilt loader metadata and launcher profiles.
- Forge installer profiles and Maven metadata.
- `Serialize` and `Deserialize` implementations for every public model.
- Optional and defaulted fields that tolerate normal upstream schema drift.

## Installation

```toml
[dependencies]
anyhow = "1"
elemental-schema = "0.2"
serde_json = "1"
```

## Quick Start

Parse a Mojang version manifest and read the latest release:

```rust
use anyhow::Result;
use elemental_schema::mojang::launcher::LaunchMetaData;

fn latest_release(raw: &str) -> Result<String> {
    let manifest: LaunchMetaData = serde_json::from_str(raw)?;
    Ok(manifest.latest.release)
}
```

Read the Java requirement from a Piston version document:

```rust
use anyhow::Result;
use elemental_schema::mojang::piston::PistonMetaData;

fn required_java(raw: &str) -> Result<usize> {
    let metadata: PistonMetaData = serde_json::from_str(raw)?;
    Ok(metadata.java_version.major_version)
}
```

## Modules

| Module | Contents |
| --- | --- |
| `mojang::launcher` | `version_manifest_v2.json` models |
| `mojang::piston` | Version metadata, arguments, rules, libraries, downloads, logging, and asset indexes |
| `fabric` | Fabric game versions, loader versions, and launcher profiles |
| `quilt` | Quilt game versions, loader versions, and launcher profiles |
| `forge` | Installer profiles, installer processors, and Maven metadata |

Quilt profile types reuse the Fabric profile library types because the two
launcher formats share the same library and logging shapes.

## Schema Notes

The models keep upstream names through `#[serde(rename = "...")]`. Fields that
are absent in older or newer manifests use `Option` or `#[serde(default)]` where
the launcher can continue without them.

Mojang version manifest timestamps are represented as `jiff::Timestamp`.
Installers, loader metadata, and version documents are available as owned
types that can be persisted or transformed without retaining the source JSON.

## Related Crates

- [elemental-core](https://github.com/ElectronicWave/Elemental/blob/main/crates/core/README.md)
  combines these models with storage, runtime discovery, authentication, and
  process launch primitives.
- [elemental-driver](https://github.com/ElectronicWave/Elemental/blob/main/crates/driver/README.md)
  fetches manifests and turns them into installable, launchable driver
  metadata.
- [elemental-kit](https://github.com/ElectronicWave/Elemental/blob/main/README.md#the-elemental-kit-facade)
  re-exports this crate behind the `schema` feature.
