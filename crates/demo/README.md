# demo

Command-line walkthrough of the Elemental launcher stack.

`demo` is a binary example that prepares and launches Minecraft instances
through the `elemental-kit` facade. It exercises every supported driver, prints
installation diagnostics, streams the game process output, and finishes with a
summary. It is intended for development and manual verification rather than as
a library dependency.

## Features

- Vanilla and nine modloader or legacy distribution flows.
- Prepare-and-launch and local-only reload modes.
- Shared storage and Java runtime selection flags.
- Built-in per-driver defaults for quick experiments.
- Installation diagnostics and streamed game logs.
- Offline credentials so local testing does not require an account.

## Quick Start

Run from the workspace root:

```bash
cargo run -p demo -- --help
cargo run -p demo -- list
cargo run -p demo -- vanilla 1.20.1
```

With no subcommand, the demo prepares and launches Fabric 1.20.1:

```bash
cargo run -p demo
```

The first run downloads metadata and artifacts into `.minecraft`. It also needs
a compatible local Java runtime.

## Commands

```text
demo [OPTIONS] [COMMAND]
```

| Command | Positionals |
| --- | --- |
| `list` | None |
| `vanilla` | `[game_version] [instance_name]` |
| `fabric` | `[game_version] [loader_version] [instance_name]` |
| `legacyfabric`, `legacy-fabric` | `[game_version] [loader_version] [instance_name]` |
| `babric` | `[game_version] [loader_version] [instance_name]` |
| `quilt` | `[game_version] [loader_version] [instance_name]` |
| `liteloader`, `lite-loader` | `[game_version] [loader_version] [instance_name]` |
| `rift` | `[game_version] [loader_version] [instance_name]` |
| `forge` | `[game_version] [loader_version] [instance_name]` |
| `cleanroom` | `[game_version] [loader_version] [instance_name]` |
| `neoforge`, `neo-forge` | `[game_version] [loader_version] [instance_name]` |

`list` scans the storage root and prints every locally recognized instance,
including its driver and path.

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
| `--runtime-executable PATH` | Use one explicit Java executable |

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
| NeoForge | 1.21.1 | 21.1.199 | `MyNeoForge-1.21.1` |

## Examples

Prepare and launch Forge with the default loader version:

```bash
cargo run -p demo -- forge 1.20.1
```

Use an isolated storage directory and reload an existing Fabric instance:

```bash
cargo run -p demo -- \
  --storage-root .minecraft-demo \
  --local-only \
  fabric 1.20.1 0.16.10 MyFabric-1.20.1
```

Select Java 21 explicitly:

```bash
cargo run -p demo -- \
  --runtime-major-version 21 \
  vanilla 1.20.1 Java21-Vanilla
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
