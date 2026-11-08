# launchpad

Command-line walkthrough of the Elemental launcher stack.

`launchpad` is a CLI that prepares and launches Minecraft instances
through the `elemental-kit` facade. It exercises every supported driver, prints
installation diagnostics, streams the game process output, and finishes with a
summary. It is intended for development and manual verification rather than as
a library dependency.

## Features

- Vanilla and ten modloader or legacy distribution flows.
- Prepare-and-launch and local-only reload modes.
- Shared storage and Java runtime selection flags.
- Built-in per-driver defaults for quick experiments.
- Interactive wizard for driver, version, instance, and Java runtime selection.
- Catalog-backed version search with live filtering and download progress bars.
- Installation diagnostics and streamed game logs.
- Offline credentials so local testing does not require an account.

## Quick Start

Run from the workspace root:

```bash
cargo run -p launchpad -- --help
cargo run -p launchpad -- list
cargo run -p launchpad -- vanilla 1.20.1
```

With no launch arguments, launchpad starts an interactive wizard:

```bash
cargo run -p launchpad
```

The wizard offers to prepare a new instance, launch an already installed
instance, or list local instances. It prompts for the driver, game version,
loader version, instance name, and Java runtime. Runtime choices come from the
configured providers: `Auto` uses the driver default when present and otherwise
falls back to the prepared version metadata.

Version choices come from each driver's live catalog. Type to filter the list,
and preparations show an animated download bar with task counts, transferred
bytes, speed, ETA, and the oldest four active task paths.

Non-interactive invocations such as `cargo run -p launchpad -- fabric 1.20.1` keep
using the driver defaults. The first run downloads metadata and artifacts into
`.minecraft`. It also needs a compatible local Java runtime.

## Commands

```text
launchpad [OPTIONS] [DRIVER] [GAME_VERSION] [LOADER_VERSION] [COMMAND]
```

| Command | Positionals |
| --- | --- |
| `list` | None |
| `vanilla` | `[game_version]` |
| `fabric` | `[game_version] [loader_version]` |
| `legacyfabric`, `legacy-fabric` | `[game_version] [loader_version]` |
| `babric` | `[game_version] [loader_version]` |
| `quilt` | `[game_version] [loader_version]` |
| `liteloader`, `lite-loader` | `[game_version] [loader_version]` |
| `rift` | `[game_version] [loader_version]` |
| `forge` | `[game_version] [loader_version]` |
| `cleanroom` | `[game_version] [loader_version]` |
| `outlands` | `[game_version] [loader_version]` |
| `neoforge`, `neo-forge` | `[game_version] [loader_version]` |

Every launch command also accepts `--instance-name NAME`.

`list` scans the storage root and prints every locally recognized instance,
including its driver and path.

## Interactive Mode

Running launchpad without a launch command opens the interactive wizard. The
wizard uses `inquire` selection and text prompts, so arrow keys and search work
as expected. Press `Esc` to return to the previous step; pressing `Esc` on the
first menu exits the wizard.

- `Prepare and launch a new instance` asks for the driver, versions, instance
  name, and Java runtime. `Auto` follows the driver default, falling back to
  version metadata; the remaining entries show each Java major found by the
  runtime providers together with the path that will be launched.
- `Launch an installed instance` scans the storage root and lets you pick from
  the detected instances before choosing a Java runtime.
- `List installed instances` behaves like `launchpad list`.

Passing `--local-only` limits the wizard to installed instances and listing.

## Global Options

| Option | Default | Behavior |
| --- | --- | --- |
| `--storage-root PATH` | `.minecraft` | Game root used for installations |
| `--local-only` | Disabled | Load and launch an already prepared instance without installing |

## Runtime Options

Runtime options are available on `vanilla` and every loader command:

| Option | Behavior |
| --- | --- |
| `--runtime-major-version N` | Select a Java major version |
| `--runtime-validation strict\|disabled` | Validate the selected Java version or skip the check |
| `--runtime-path PATH` | Prepend a custom runtime provider; repeatable |

Custom runtime providers are prepended before the platform defaults. Cleanroom
defaults to Java 25 when no major version is supplied.

## Driver Defaults

| Driver | Game | Loader | Instance |
| --- | --- | --- | --- |
| Vanilla | 1.20.1 | - | `MyVanilla-1.20.1` |
| Fabric | 1.20.1 | 0.16.10 | `MyFabric-1.20.1` |
| Legacy Fabric | 1.20.1 | 0.16.10 | `MyLegacyFabric-1.20.1` |
| Babric | 1.20.1 | 0.16.10 | `MyBabric-1.20.1` |
| Quilt | 1.20.1 | 0.24.0 | `MyQuilt-1.20.1` |
| LiteLoader | 1.7.10 | 1.7.10_04 | `MyLiteLoader-1.7.10` |
| Rift | 1.13.2 | 1.0.4-106 | `MyRift-1.13.2` |
| Forge | 1.20.1 | 47.3.1 | `MyForge-1.20.1` |
| Cleanroom | 1.12.2 | 0.5.8-alpha | `MyCleanroom-1.12.2` |
| Outlands | 1.12.2 | 2026.11.06 | `MyOutlands-1.12.2` |
| NeoForge | 1.21.1 | 21.1.199 | `MyNeoForge-1.21.1` |

## Examples

Prepare and launch Forge with the default loader version:

```bash
cargo run -p launchpad -- forge 1.20.1
```

Use an isolated storage directory and reload an existing Fabric instance:

```bash
cargo run -p launchpad -- \
  --storage-root .minecraft-launchpad \
  --local-only fabric 1.20.1 0.16.10 \
  --instance-name MyFabric-1.20.1
```

Select Java 21 explicitly:

```bash
cargo run -p launchpad -- \
  --runtime-major-version 21 \
  vanilla 1.20.1 --instance-name Java21-Vanilla
```

## Behavior

- Every launch uses the offline username `Player`.
- Preparation reports catalog, metadata, download, and runtime diagnostics.
- Game stdout and stderr are streamed to the terminal.
- `--local-only` skips preparation and fails if the requested instance is not
  already installed.

> [!NOTE]
> Offline credentials are suitable only for local development. Use a real
> `Authorizer` integration when building a player-facing launcher.

For the library API behind this example, see the
[elemental-launcher README](https://github.com/ElectronicWave/Elemental/blob/main/crates/launcher/README.md)
and the
[workspace README](https://github.com/ElectronicWave/Elemental/blob/main/README.md).
