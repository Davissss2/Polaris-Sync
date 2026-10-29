# Agent guide

Polaris is a Rust workspace: a CLI that syncs a Roblox project on disk with
Roblox Studio. This file tells agents how to work on the repository. To use
the tool from an agent, read [skills/polaris/SKILL.md](skills/polaris/SKILL.md).

## Layout

| Path | Purpose |
| --- | --- |
| `src/cli` | One module per command |
| `src/core` | Tree, snapshots, processor (sync engine) |
| `src/middleware` | File formats: Luau, JSON, models, data |
| `src/server` | Local HTTP/MessagePack API |
| `src/config.rs` | Settings and `polaris.toml` |
| `src/templates.rs` | Templates bundled with `include_dir` |
| `assets/templates` | Template sources (`game`, `place`, `model`, ...) |
| `plugin` | Roblox Studio plugin (Luau, Rojo + Wally) |
| `skills` | Agent skills |

## Commands

```bash
cargo check --all-targets
cargo test
cargo build
./target/debug/polaris --help
```

The plugin is built separately, with the CLI (never with Rojo):

```bash
cd plugin
aftman install
wally install
cd ..
./target/debug/polaris build plugin
```

## Rules

1. Templates are compiled into the binary. After editing
   `assets/templates/**`, rebuild; there is no install step.
2. Every new mapped service must exist in `DATA_MODEL_SERVICES` and pass
   `cargo test` (a test validates them against the Roblox reflection database).
3. New commands need `--help` text, a `--json` branch and an entry in the
   README command table.
4. Keep user-facing output English, stable and free of colors in JSON mode.
5. Do not add telemetry or network calls on startup; the CLI must work
   offline and without touching the network unless the user asks for it.
6. Do not rename the `PolarisEmpty` property or the MessagePack protocol
   without updating `plugin/src/Core/Snapshot.luau` and the CLI together.
7. Preserve the Apache-2.0 license and the attribution in `NOTICE`.
8. Never build the plugin with Rojo. `plugin/src/Lib/Dom/database.msgpack` is
   MessagePack, which Rojo skips without failing, so the plugin dies on load.
   `polaris build` converts it into a ModuleScript; CI builds the plugin that
   way and fails when the build breaks.
9. Model files (`.model.json`, `.rbxm`, `.rbxmx`) are read only. Their instances
   cannot be written back into the file, so an edit made in Studio is dropped.
   Anything the user is expected to edit belongs in the directory format
   (`<Name>/init.meta.json`), which both reads and writes.

## Definition of done

- `cargo check --all-targets` clean.
- `cargo test` green.
- Manual check with the real binary (`init`, `workspace expand`,
  `list --json`) when the change is user facing.
- README and CHANGELOG updated when behavior changes.
