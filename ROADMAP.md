# Roadmap

This roadmap tracks Elemental from the `0.1.0` release toward a stable `0.2.0`
kernel and the work planned after it. The GitHub issue tracker is the source of
truth for individual tasks. This document explains priority, sequencing, and
what "done" means.

Last reviewed: 2026-11-06.

## Scope

Elemental is a modular Rust toolkit for Minecraft launchers. The roadmap is
ordered around four concerns:

1. Make the kernel API coherent enough to release as `0.2.0`.
2. Keep loader support evidence-based instead of claiming untested ranges.
3. Grow around real ecosystem needs: addons, mod metadata, accounts, and packs.
4. Expose the kernel through front ends only after its boundaries are stable.

## Current State

- `0.1.0` is the latest published release.
- The workspace is preparing `0.2.0` under milestone
  [#1](https://github.com/ElectronicWave/Elemental/milestone/1).
- The workspace is split into `core`, `driver`, `launcher`, `infra`, `shared`,
  `object`, `schema`, `elemental-kit`, and the `launchpad` CLI.
- Catalog, inspect, install, load-installed, and launch are implemented for
  every driver listed below.

### Drivers

| Family | Drivers | Implementation state |
| --- | --- | --- |
| Vanilla | Vanilla | Verified across legacy and modern launch arguments |
| Fabric profile | Fabric, LegacyFabric, Babric, Quilt | Verified on representative anchors |
| Installer | Forge, NeoForge, CleanroomMC, Outlands | Implemented; Forge, NeoForge, and Cleanroom have runtime anchors, Outlands still needs one |
| Profiled legacy | Rift, LiteLoader | Verified on one anchor each |

Verified runtime anchors currently recorded:

| Driver | Anchor |
| --- | --- |
| Vanilla | `1.8.9`, `1.12.2`, `1.16.5`, `1.20.1` |
| Fabric | `1.14.4`, `1.16.5`, `1.20.1` |
| LegacyFabric | `1.13.2` |
| Babric | `b1.7.3` |
| Quilt | `1.20.1` |
| Forge | `1.12.2 / 14.23.5.2860`, `1.20.1 / 47.3.1` |
| NeoForge | `1.21.1 / 21.1.199` |
| CleanroomMC | `1.12.2 / 0.5.8-alpha` |
| Rift | `1.13.2 / 1.0.4-106` |
| LiteLoader | `1.7.10 / 1.7.10_04` |
| Outlands | Not recorded yet |

Anchors are verified points, not claims for the full upstream support range.
Rolling targets such as the latest release and latest stable loader remain
recurring regression checks.

### Landed Recently

- Error handling now uses `thiserror` for libraries and `anyhow` for
  applications ([#206](https://github.com/ElectronicWave/Elemental/issues/206)).
- The downloader was rebuilt around explicit sessions, configurable retry,
  concurrency, rate limits, and report handling
  ([#217](https://github.com/ElectronicWave/Elemental/issues/217)).
- Java runtime providers are complete
  ([#10](https://github.com/ElectronicWave/Elemental/issues/10)).
- The object pool semantics were corrected
  ([#200](https://github.com/ElectronicWave/Elemental/issues/200)).
- Persistent data can migrate between different struct shapes
  ([#195](https://github.com/ElectronicWave/Elemental/issues/195)).
- Outlands joined the installer family
  ([#208](https://github.com/ElectronicWave/Elemental/issues/208)).

## Milestone 0.2.0

Goal: release a coherent kernel API. This milestone is not about adding more
drivers; it is about removing rough edges that would otherwise freeze into the
public API.

### Release Blockers

| Issue | Area | Why it blocks the release |
| --- | --- | --- |
| [#193](https://github.com/ElectronicWave/Elemental/issues/193) | Versions | Minecraft's year-based and patch-based version naming must be handled consistently before new upstream versions accumulate |
| [#198](https://github.com/ElectronicWave/Elemental/issues/198) | Infra | The session-based downloader redesign has landed; the remaining ergonomic gaps need to be closed or explicitly deferred |
| [#205](https://github.com/ElectronicWave/Elemental/issues/205) | Core | Yggdrasil authentication is required for non-Microsoft legacy accounts |
| [#209](https://github.com/ElectronicWave/Elemental/issues/209) | Driver | Creating a new family or modloader must not require copying internal glue |
| [#210](https://github.com/ElectronicWave/Elemental/issues/210) | Launcher | The high-level launcher API should be the easy path, not a thin wrapper over driver internals |

### Related Work

- [#216](https://github.com/ElectronicWave/Elemental/issues/216) make the
  `reqwest` client injectable so hosts can control proxies, TLS, timeouts, and
  observability.
- [#219](https://github.com/ElectronicWave/Elemental/issues/219) replace the
  legacy roadmap with this document.

### Release Criteria

- Every blocker above is closed or explicitly moved to a later milestone.
- `cargo fmt` and the workspace test suite pass.
- Every driver has a recorded smoke anchor for its current implementation path.
- README support matrices and crate documentation match the actual driver list.
- The public API no longer exposes family internals that hosts must stitch
  together themselves.

## After 0.2.0

### 1. Stability and Coverage

Issue: [#126](https://github.com/ElectronicWave/Elemental/issues/126).

Turn the verified anchors into a repeatable compatibility matrix:

- record catalog, inspect, install, load, and launch results per driver
- add rolling checks for the latest Minecraft release and latest loader version
- validate additional generations for LegacyFabric, Babric, Quilt, Forge,
  NeoForge, CleanroomMC, Rift, and LiteLoader
- add the first Outlands runtime anchor

Success means support claims are backed by smoke evidence and regressions are
found before users report them.

### 2. Addons

Issue: [#128](https://github.com/ElectronicWave/Elemental/issues/128).

Addons are overlays on an existing instance, not top-level drivers. The first
target is OptiFine, followed by OptiFabric and similar patch systems.

The addon layer should support:

- applying an addon to a prepared instance
- recording which addon version is active
- removing or replacing an addon without rebuilding the base instance
- leaving base-driver install and launch semantics unchanged

### 3. Mod Metadata and Packs

Issues: [#13](https://github.com/ElectronicWave/Elemental/issues/13),
[#59](https://github.com/ElectronicWave/Elemental/issues/59),
[#45](https://github.com/ElectronicWave/Elemental/issues/45).

Build the data layer needed by modpack workflows:

- parse mod metadata across loader formats
- resolve mod dependency relationships and version constraints
- integrate Packust for pack manifests and instance materialization

This work should stay separate from the launcher kernel unless a concrete
workflow proves that it belongs there.

### 4. Accounts and Skins

Issues: [#205](https://github.com/ElectronicWave/Elemental/issues/205),
[#11](https://github.com/ElectronicWave/Elemental/issues/11).

- finish Yggdrasil support alongside the existing offline and Microsoft
  authorizers
- add skin lookup, upload, and profile metadata support
- keep authentication pluggable so host applications can supply their own
  authorizer

### 5. Observability

Issue: [#74](https://github.com/ElectronicWave/Elemental/issues/74).

Move beyond ad hoc logging:

- emit structured spans and events for catalog, download, install, and launch
- propagate tracing context through async download and install tasks
- make downloader and launcher reports observable without parsing strings

### 6. Networking Control

Issue: [#216](https://github.com/ElectronicWave/Elemental/issues/216).

Allow hosts to supply a configured `reqwest` client. This covers proxies,
custom TLS roots, timeouts, user agents, and instrumentation while keeping
mirror and origin rewriting inside the driver layer.

## Architecture Direction

The current family split is working and should be preserved:

| Family | Responsibility |
| --- | --- |
| `version_json` | Direct metadata, libraries, assets, and launch arguments |
| fabric-like | Profile merging for Fabric, LegacyFabric, Babric, and Quilt |
| installer | Running installers and applying their profiles and processors |
| profiled legacy | Loaders whose embedded profiles still normalize into the same installation model |
| addon | Patch and overlay behavior layered on a base driver |

Do not introduce a separate `legacy` family unless a target proves it needs
relaunch, bootstrap indirection, or library mutation that the current profile
pipeline cannot express.

Instance changes should be modeled as installation mutation, not a universal
`Driver::uninstall()` method:

- change the loader version inside a family
- remove the loader layer and return to Vanilla
- repair missing or invalid artifacts
- reinstall the current driver state

## Deferred

- GUI work, until the kernel API and addon model are stable.
- Broad version-range claims without smoke anchors.
- A universal loader-uninstall abstraction.
- Pulling modpack resolution into the launcher core.

## Priority Order

1. Close the `0.2.0` blockers and ship the kernel API.
2. Establish the stability matrix and first Outlands anchor.
3. Deliver the addon layer with OptiFine.
4. Build mod metadata, dependency, and Packust integration.
5. Finish account and skin support.
6. Add tracing and networking control.
7. Build front ends on top of the stable kernel.

Dependency dashboard updates
([#79](https://github.com/ElectronicWave/Elemental/issues/79)) are handled by
Renovate and are not roadmap work.
