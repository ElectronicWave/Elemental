# Roadmap

## Scope

This document summarizes:

- the current Elemental driver state
- the version ranges that can be claimed now
- upstream findings about Fabric, Forge, OptiFine, and CleanroomMC
- a recommended execution plan for the next driver families

It is intended as a working architecture reference, not as a release guarantee.

## Current State

| Family / Driver | Catalog | Inspect | Install | Load Installed | Launch | Notes                                                                                                                                                                    |
| --------------- | ------- | ------- | ------- | -------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Vanilla         | Yes     | Yes     | Yes     | Yes            | Yes    | Current primary complete driver, smoke-verified on representative release anchors                                                                                        |
| Fabric-like     | Yes     | Yes     | Yes     | Yes            | Yes    | Shared flavor-aware profile family now drives multiple loader variants                                                                                                   |
| Fabric          | Yes     | Yes     | Yes     | Yes            | Yes    | Modern Fabric mainline implemented and smoke-verified on representative anchors                                                                                          |
| LegacyFabric    | Yes     | Yes     | Yes     | Yes            | Yes    | End-to-end flow is working and smoke-verified on a representative legacy anchor                                                                                          |
| Babric          | Yes     | Yes     | Yes     | Yes            | Yes    | End-to-end flow is working and smoke-verified on a representative beta anchor                                                                                            |
| Quilt           | Yes     | Yes     | Yes     | Yes            | Yes    | Independent fabric-like driver implemented and smoke-verified on a representative anchor                                                                                 |
| Forge           | Yes     | Yes     | Yes     | Yes            | Yes    | Installer-family driver now reaches a verified modern launch anchor                                                                                                      |
| NeoForge        | Yes     | Yes     | Yes     | Yes            | Yes    | Installer-family driver now reaches a verified modern launch anchor; catalog game-version grouping remains heuristic, but now covers both pre-2026 and year-based naming |
| CleanroomMC     | Yes     | Yes     | Yes     | Yes            | Yes    | Installer-family driver is implemented and smoke-verified on a `1.12.2 / 0.5.8-alpha` anchor                                                                             |
| LiteLoader      | No      | No      | No      | No             | No     | Not started                                                                                                                                                              |
| Rift            | Yes     | Yes     | Yes     | Yes            | Yes    | Direct profiled `version_json` driver is implemented against official release jars; runtime smoke coverage still needs to be recorded                                     |
| OptiFine        | No      | No      | No      | No             | No     | Not started                                                                                                                                                              |

## Verified Smoke Coverage

The current workspace has already passed end-to-end smoke validation on the following anchor versions.

These anchors should be read as verified points inside the support range, not as the entire range by themselves.

| Family / Driver | Verified anchors                           | Notes                                                                                                                                                                       |
| --------------- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Vanilla         | `1.8.9`, `1.12.2`, `1.16.5`, `1.20.1`      | Covers old `minecraftArguments` flow and the modern `arguments` flow in the current workspace                                                                               |
| Fabric          | `1.14.4`, `1.16.5`, `1.20.1`               | Confirms the modern Fabric profile flow across early, mid, and current supported release lines                                                                              |
| LegacyFabric    | `1.13.2`                                   | Confirms the flavor-aware family path on the modern edge of the LegacyFabric-supported band                                                                                 |
| Babric          | `b1.7.3`                                   | Confirms the flavor-aware family path on a representative beta-era Babric anchor                                                                                            |
| Quilt           | `1.20.1`                                   | Confirms a second independent fabric-like driver on the shared profile-driven substrate                                                                                     |
| Forge           | `1.12.2 / 14.23.5.2860`, `1.20.1 / 47.3.1` | Confirms the installer-family pipeline across a classic legacy-era anchor and a modern Forge anchor                                                                         |
| NeoForge        | `1.21.1 / 21.1.199`                        | Confirms the installer-family pipeline on a modern NeoForge anchor; catalog grouping is still version-name heuristic, but it now covers both pre-2026 and year-based naming |
| CleanroomMC     | `1.12.2 / 0.5.8-alpha`                     | Confirms the installer-family pipeline on a Java 25-era Cleanroom anchor after legacy runtime cleanup                                                                       |

Rolling targets such as the latest release, latest snapshot, and latest stable loader should still be treated as recurring regression checks rather than one-time milestones.

## Claimed Version Range

These are the ranges I would claim today based on the current code, upstream docs, and the verified smoke anchors above.

| Family / Driver | Range to claim now                                                                    | Confidence   | Why                                                                                                                                                                                                                              |
| --------------- | ------------------------------------------------------------------------------------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Vanilla         | `1.8.9+` release line, including modern mainline releases                             | High         | The current workspace now has successful smoke anchors on `1.8.9`, `1.12.2`, `1.16.5`, and `1.20.1`, covering both legacy and modern launch argument paths                                                                       |
| Vanilla         | current snapshot line                                                                 | Medium-High  | Modern `version_json` handling is in place, but latest snapshot should continue to be treated as a rolling regression target                                                                                                     |
| Vanilla         | `1.6.1` to `1.7.x`                                                                    | Medium       | The builder now supports legacy JVM and game argument handling, but this older band still lacks the same smoke coverage as `1.8.9+`                                                                                              |
| Vanilla         | `<1.6`                                                                                | Do not claim | Current metadata assumptions still lean on modern Piston-style fields                                                                                                                                                            |
| Fabric          | modern official Fabric line, practically `1.14.4+`                                    | High         | The current workspace has successful smoke anchors on `1.14.4`, `1.16.5`, and `1.20.1`, and the implementation consumes official Fabric profile metadata                                                                         |
| LegacyFabric    | `1.13.2` verified, broader range not claimed yet                                      | Medium       | The current workspace now has an end-to-end verified anchor on `1.13.2`, but older LegacyFabric-supported releases and snapshots still need systematic smoke coverage                                                            |
| Babric          | `b1.7.3` verified, broader range not claimed yet                                      | Medium       | The current workspace now has an end-to-end verified anchor on `b1.7.3`, but broader Babric-supported beta coverage still needs systematic smoke coverage                                                                        |
| Quilt           | `1.20.1` verified, broader range not claimed yet                                      | Medium       | The current workspace now has an end-to-end verified anchor on `1.20.1`, but broader Quilt-supported version coverage still needs systematic smoke coverage                                                                      |
| Forge           | `1.12.2 / 14.23.5.2860` and `1.20.1 / 47.3.1` verified, broader range not claimed yet | High         | The installer-family pipeline now has verified anchors on both a classic `1.12.2` generation and a modern `1.20.1` generation, but broader Forge coverage still needs systematic validation                                      |
| NeoForge        | `1.21.1 / 21.1.199` verified, broader range not claimed yet                           | High         | The installer-family pipeline now has a verified modern NeoForge anchor, but broader NeoForge coverage still needs systematic validation and catalog grouping still relies on version-name heuristics rather than upstream truth |
| CleanroomMC     | `1.12.2 / 0.5.8-alpha` verified, broader range not claimed yet                        | High         | The installer-family pipeline now has a verified Cleanroom anchor on the only currently targeted Minecraft line, but broader Cleanroom release coverage and companion-pack semantics still need systematic validation            |
| Rift            | official `1.13` and `1.13.2` release line is implemented, but do not claim yet      | Low-Medium   | The direct profiled driver now derives metadata from official release jars and normalizes their embedded launcher profiles, but no runtime smoke anchor has been recorded yet                                              |

## Upstream Findings

## Fabric

Confirmed:

- Fabric officially supports most snapshots from `18w43b+` and releases `1.14+`.
- Fabric installation is profile-oriented. The official installer creates a new launcher version/profile.
- Fabric Meta provides launcher-facing metadata endpoints such as profile JSON and related version endpoints.

Relevant sources:

- Fabric FAQ: <https://wiki.fabricmc.net/faq:user>
- Fabric install docs: <https://wiki.fabricmc.net/install>
- Fabric third-party launcher flow: <https://wiki.fabricmc.net/player:tutorials:third-party:prism>

Architecture implication:

- Fabric belongs in a `fabric-like` or `version_json-derived` family.
- It should not be modeled as an installer-first family like modern Forge.

## Forge

Confirmed:

- `forge-install-bootstrapper` exists specifically to automate Forge installer execution after Forge removed the `--installClient` CLI path.
- The project explicitly describes itself as supporting installer automation for any version that still ships an installer since `1.5.2`.

Relevant source:

- Forge install bootstrapper: <https://github.com/Steve-xmh/forge-install-bootstrapper>

Architecture implication:

- Forge is installer-driven.
- It should be modeled as an `installer family`, not as a plain `version_json` family member.
- Elemental can now safely claim one classic Forge anchor and one modern Forge anchor, but should still avoid broader range claims until more installer generations are validated.

## OptiFine

Confirmed:

- `optifine-installer` is not just a metadata downloader.
- Its documented flow is instance-mutating:
  - copy Minecraft version
  - install OptiFine library
  - install LaunchWrapper library for newer branches
  - update version JSON
  - update launcher state files
- The project explicitly claims coverage for almost all `1.7.2+` OptiFine versions.

Relevant source:

- OptiFine installer: <https://github.com/Steve-xmh/optifine-installer>

Architecture implication:

- OptiFine should not be treated as a normal top-level driver first.
- It is better modeled as an addon or patch-installer family.

## CleanroomMC

Confirmed:

- Cleanroom targets `1.12.2 on Java 25+`.
- Cleanroom defines:
  - `CleanroomLoader` as a continuation and revamp of ForgeModLoader
  - `Cleanroom Minecraft` as a continuation and revamp of MinecraftForge
  - `Foundation` as a LaunchWrapper replacement
- Official launcher guidance prefers MultiMC-based launchers and MMC instance import.
- Standard launchers are supported through a relauncher or installer path.
- Cleanroom can also relaunch from a Forge `1.12.2` instance.

Relevant sources:

- Cleanroom README: <https://github.com/CleanroomMC/Cleanroom>
- Cleanroom client install docs: <https://cleanroommc.com/wiki/end-user-guide/installation/install-client>

Architecture implication:

- Cleanroom is not `fabric-like`.
- Its published installer artifact fits naturally into the `installer` family.
- The current workspace now treats Cleanroom as an installer-family driver on the `1.12.2` line, while MMC import semantics and wider companion-pack handling remain future work.

## Repositories Mentioned But Not Directly Verified

The following names were provided as useful family indicators, but I did not successfully open public repository pages or READMEs for them during this research pass:

- `loomboot4r`
- `legacyboot4r`
- `cleanboot4r`
- `anvilboot4r`
- `spzboot4r`

They are still useful as architecture hints, but any statement about them in this document should be treated as family inference rather than directly confirmed repository evidence.

## Recommended Family Model

Elemental should not keep growing as a flat list of unrelated drivers.

The cleaner long-term model is:

| Layer    | Responsibility                                                      |
| -------- | ------------------------------------------------------------------- |
| `Driver` | User-facing distribution semantics and instance lifecycle           |
| `Family` | Shared install/boot protocol for a group of drivers                 |
| `Core`   | Storage, layout, runtime, launch primitives, downloader integration |

Recommended families:

| Family                | Examples                            | Character                                                                                |
| --------------------- | ----------------------------------- | ---------------------------------------------------------------------------------------- |
| `version_json` family | Vanilla, Rift                       | Modern metadata-driven install and launch, including direct launcher-profile derivatives  |
| `fabric-like` family  | Fabric, LegacyFabric, Babric, Quilt | Profile-driven or version-json-derived boot                                              |
| `installer` family    | Forge, NeoForge, CleanroomMC        | Installer protocol and materialization, including legacy-derived installer distributions |
| `legacy` family       | LiteLoader                          | LaunchWrapper, tweaker, relaunch, and bootstrap paths that cannot be expressed as direct profile-driven metadata |
| `addon` family        | OptiFine, OptiFabric                | Patch or addon semantics layered on top of a base driver                                 |

## Should Elemental Support Driver Uninstall

Yes, but not as a single universal `Driver::uninstall()` operation.

The pattern used by other launchers is usually:

- remove a component
- change loader version
- switch back to Vanilla
- repair or reinstall the instance

instead of exposing one abstract "uninstall driver" action.

### What Other Launchers Do

#### Prism Launcher

Prism models loaders as components in the instance version page.

Confirmed behavior:

- users can change the version
- change the load order
- remove components

Relevant sources:

- Prism version page: <https://prismlauncher.org/wiki/help-pages/instance-version/>
- Prism instance architecture: <https://www.mintlify.com/PrismLauncher/PrismLauncher/development/architecture/instance-management>

Implication:

- Prism treats loader removal as component graph editing, not as a driver-level uninstall method

#### ATLauncher

ATLauncher exposes loader version change operations, not a generic uninstall abstraction.

Relevant source:

- ATLauncher updating mod loader version: <https://wiki.atlauncher.com/guides/updating-mod-loader-version/>

Implication:

- the model is "change the loader version" rather than "call uninstall on the loader"

#### CurseForge

CurseForge exposes modloader version selection inside profile options.

Relevant source:

- CurseForge support article: <https://support.curseforge.com/support/solutions/articles/9000230030-changing-the-mod-loader-version-of-a-modpack-or-custom-profile>

Implication:

- the product model is configuration change, not a generic uninstall protocol

#### Modrinth

Modrinth exposes installation-state operations such as:

- switch between vanilla and modded
- change game version
- repair installation
- reinstall
- unlink modpack

Relevant source:

- Modrinth content management overhaul: <https://modrinth.com/news/article/content-management-overhaul/>

Implication:

- the model is installation mutation and repair, not a single uninstall verb

### Recommended Elemental Model

Elemental should support loader removal and rollback behaviors, but the API should be framed as instance installation mutation.

Recommended operations:

| Operation             | Meaning                                                                                |
| --------------------- | -------------------------------------------------------------------------------------- |
| `change_loader`       | change loader version inside the same family                                           |
| `remove_loader`       | convert an instance back to Vanilla or remove the active loader layer                  |
| `repair_installation` | reinstall missing or invalid launcher artifacts without changing the instance identity |
| `reinstall`           | rebuild the installed state for the current driver or family                           |

### Why Not `Driver::uninstall()`

Family semantics are too different:

- `fabric-like` loaders behave more like removable components or profile overlays
- `installer` families such as Forge are closer to re-materialization than clean uninstall
- `legacy` families may involve tweakers, relaunchers, or jar-era patch flows
- `addon` families such as OptiFine are even less suitable as top-level uninstall targets

Because of this, a single trait method like:

```rust
async fn uninstall(&self, instance: &Instance) -> Result<()>
```

would likely become a leaky abstraction.

### Recommendation

- support loader removal
- do not model it as a universal driver trait method
- model it as explicit instance installation mutation capabilities

## Recommended Execution Plan

## Phase 1: Turn Estimates Into Verified Facts

Goal:

- replace guessed support ranges with real smoke validation

Work:

- Vanilla smoke matrix:
  - `1.8.9` ✅
  - `1.12.2` ✅
  - `1.16.5` ✅
  - `1.20.1` ✅
  - latest release as a rolling regression target
  - latest snapshot as a rolling regression target
- Fabric smoke matrix:
  - `1.14.4` ✅
  - `1.16.5` ✅
  - `1.20.1` ✅
  - latest stable loader as a rolling regression target

Output:

- validated support table
- issue list for broken ranges

Current status:

- anchor-version smoke validation is complete for the currently targeted Vanilla and Fabric release points
- Phase 1 is good enough to unblock Phase 2 work
- latest release, latest snapshot, and latest stable loader should continue to be exercised as recurring checks

## Phase 2: Finish Fabric-Like Family

Goal:

- make the second family real, not just Fabric-only

Work:

- stabilize modern Fabric with real runtime validation
- finish the `fabric-like` family substrate so flavor semantics stay aligned with the real upstream ecosystems
- keep Modern Fabric green under recurring regression checks
- keep LegacyFabric green after the `1.13.2` verified anchor
- continue active runtime validation for:
  - Babric
- implement Quilt as an independent fabric-like driver
- verify Quilt on at least one representative stable anchor

Why this phase matters:

- it proves the family abstraction is real
- it forces the API to support multiple profile-driven loaders without collapsing back into Fabric-specific logic

Current status:

- Modern Fabric is stable on the currently verified release anchors
- LegacyFabric now has a verified end-to-end anchor at `1.13.2`
- Babric now has a verified end-to-end anchor at `b1.7.3`
- Quilt now has a verified end-to-end anchor at `1.20.1`
- the `fabric-like` family is now real enough to stop being a Fabric-only abstraction
- Phase 2 is now good enough to stop being the active roadmap bottleneck

## Phase 3: Create Installer Family

Goal:

- support installer-driven loaders without pretending they are just metadata variants

Work:

- define installer-family substrate
- support:
  - installer metadata extraction
  - artifact materialization
  - state tracking
  - instance output
- implement Forge install flow first
- follow with NeoForge

Why this phase matters:

- modern Forge is one of the strongest proofs that not everything belongs in `version_json`

Current status:

- the installer-family substrate exists
- Forge has verified launch anchors at `1.12.2 / 14.23.5.2860` and `1.20.1 / 47.3.1`
- NeoForge has a verified launch anchor at `1.21.1 / 21.1.199`
- CleanroomMC has a verified launch anchor at `1.12.2 / 0.5.8-alpha`
- NeoForge catalog grouping still uses version-name heuristics rather than upstream truth, but it now recognizes both pre-2026 and year-based naming
- broader Forge, NeoForge, and Cleanroom coverage is still unclaimed until more installer generations are smoke-validated
- Phase 3 is now good enough to stop being the primary roadmap milestone; the remaining work is wider coverage, not missing family substrate

## Phase 4: Revisit Legacy Boot Surface

Goal:

- support the remaining genuinely legacy loaders without inventing a family boundary too early

Work:

- keep Rift on the direct profiled `version_json` path unless real boot semantics prove otherwise
- runtime-smoke the official Rift line on one representative anchor first
- only introduce a separate `legacy` family if a target truly needs:
  - relaunch flows
  - bootstrap indirection beyond an embedded launcher profile
  - library/bootstrap mutation that the current profile pipeline cannot model
- use LiteLoader as the first serious pressure test if that boundary is still needed

Why this phase matters:

- this is still the step that determines whether Elemental needs another kernel family, but Rift no longer has to be the proof target for it

Current status:

- Rift is now implemented as a direct profiled `version_json` driver with catalog, inspect, install, load-installed, and launch support
- no separate `legacy` family exists yet in the current workspace
- LiteLoader remains the strongest candidate for deciding whether a dedicated legacy substrate is actually necessary

## Phase 5: Broaden CleanroomMC Coverage

Goal:

- keep Cleanroom green on the current installer-family path while deciding whether it needs a second distribution path beyond installer materialization

Work:

- keep the `1.12.2` installer-driven path healthy under recurring regression checks
- validate more Cleanroom releases when stable installer artifacts are available
- decide whether MMC instance import should be added as a secondary path instead of a prerequisite
- model companion pieces such as Fugue and Scalar explicitly if they prove necessary for wider pack compatibility

Why this phase matters:

- Cleanroom proves that installer-family drivers can still need substantial legacy runtime cleanup without becoming a fake top-level family of their own

Current status:

- a `CleanroomDriver` now exists on top of the installer-family substrate
- `1.12.2 / 0.5.8-alpha` is smoke-verified with Java 25
- broader Cleanroom release coverage and wider pack semantics are still unclaimed

## Phase 6: Add Addon Family

Goal:

- support patch-like or overlay-like systems without polluting top-level driver semantics

Work:

- define addon or patch-installer layer
- start with OptiFine
- then consider OptiFabric or other overlays

Why this phase matters:

- it avoids turning every patch system into a fake top-level driver

## Phase 7: CLI and GUI

Goal:

- expose the kernel once family boundaries are stable

Work:

- CLI after at least three families have one production-grade target each
- GUI after the CLI and install model settle

Why this phase should be late:

- building UI too early freezes unstable API and family boundaries

## Immediate Next Milestone

The next concrete milestone should be to harden the new Rift path with a real runtime smoke anchor, then use a harder target to decide whether a separate `legacy` family is justified at all.

Recommended first slice:

- verify one representative Rift anchor end-to-end through catalog, inspect, install, load-installed, and launch
- prefer `1.13.2 / 1.0.4-106` as the first anchor, with `1.13 / 1.0.4-105` as the follow-up compatibility check
- keep Rift on the direct profiled path unless the smoke work exposes a real semantic gap
- use LiteLoader, not Rift, as the first target if a dedicated `legacy` family still looks necessary afterward

Not part of the first slice:

- broad Rift range claims
- full LiteLoader support
- addon layering such as OptiFine
- CLI or GUI work

Success criteria:

- the Rift driver reaches a verified end-to-end runtime anchor
- any later `legacy` family work is justified by real unmet semantics rather than by loader era naming
- the existing `fabric-like`, direct `version_json`, and `installer` paths remain stable while Rift is hardened

## Priority Recommendation

If only one direction should be chosen next, the order should be:

1. runtime-smoke and harden Rift
2. decide whether LiteLoader truly needs a separate `legacy` family
3. broaden installer-family coverage for Forge, NeoForge, and CleanroomMC
4. add the `addon` family starting with OptiFine
5. ecosystem features such as skins, profiles, and account polish
6. CLI or GUI

Reason:

- `fabric-like` and `installer` are now real enough to stop being the immediate proof target
- Rift proved to be a direct profiled driver rather than a forced separate family
- LiteLoader is now the better discriminator for whether a real `legacy` family is needed
- `addon` work will be easier once the remaining legacy boundary is either justified or explicitly unnecessary
- the current strongest differentiator in Elemental is still the launcher kernel architecture, not front-end packaging

## Summary

The current Elemental kernel is already in a strong position:

- `Storage + Layout` survived intact
- `core` is no longer bound to Mojang-specific world assumptions
- `Vanilla` is a usable mainline driver
- `Fabric` is attached as the second real family-backed driver
- the installer family now hosts verified Forge, NeoForge, and Cleanroom anchors
- Rift now exists as a direct profiled `version_json` driver over official release jars

The next milestone is not UI polish.

The next milestone is no longer proving that `fabric-like`, direct profiled `version_json`, or `installer` can exist.

Those lines are already real enough in the current workspace.

The next milestone is proving that the new Rift path is operational on a real runtime anchor, then deciding whether harder legacy loaders actually require another family boundary.

After that, the natural order is:

- broaden installer-family coverage
- then decide the remaining legacy substrate question
- then add `addon`

That is the step that turns it from a modern launcher SDK into a broader launcher kernel without inventing unnecessary family layers.
