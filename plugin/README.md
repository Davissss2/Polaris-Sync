# Polaris Studio plugin

Roblox Studio plugin for [Polaris](https://github.com/Davissss2/Polaris-Sync),
the Roblox sync tool. It talks to the Polaris CLI and keeps the open place
in sync with the project on disk.

This is a fork of the [Argon](https://github.com/argon-rbx/argon-roblox)
plugin (Apache-2.0), rebranded for Polaris.

## Build

The plugin is built with the Polaris CLI, **not** with Rojo:
`src/Lib/Dom/database.msgpack` holds the Roblox reflection database, and Rojo
silently skips MessagePack files, which produces a plugin that fails on load
with `database is not a valid member of ModuleScript "Polaris.Lib.Dom"`.

Requirements: [Aftman](https://github.com/LPGhatguy/aftman) (or Wally installed
manually) and a Rust toolchain.

```bash
cargo build
cd plugin
aftman install
wally install
cd ..
./target/debug/polaris build plugin   # writes Polaris.rbxm at the repository root
```

Then install it:

```bash
# Option A: download the build published with the latest release
polaris plugin install

# Option B: use the file you just built
polaris plugin install Polaris.rbxm

# Option C: copy it manually
#   Windows: %LOCALAPPDATA%\Roblox\Plugins\Polaris.rbxm
```

`polaris plugin uninstall` removes it again, and `--output <path>` installs it
somewhere else.

## Usage

1. `polaris serve` in your project.
2. Open the place in Roblox Studio.
3. Click **Polaris** in the Plugins tab and connect to the session.

If the server runs with `polaris serve --secure`, paste the printed token in
the plugin settings under **Session Token** before connecting.

## Before you publish your own build

`src/App/Assets.json` still points at the icon and banner asset ids that
came with the fork. Replace them with your own uploaded decals so the
plugin shows your branding.

Mesh hydration also depends on `rbxasset://argon/*` templates shipped by the
upstream installer (`src/Core/Processor/Write.luau`). Vendor equivalent
assets and update that path before releasing a build that must be fully
independent from the upstream installation.

## Protocol

The plugin speaks the same MessagePack protocol as the Polaris CLI
(`/subscribe`, `/read`, `/write`, `/exec`), so any 2.x compatible server
works. Empty property maps are flagged with `PolarisEmpty`.

## Layout

| Path | Purpose |
| --- | --- |
| `src/init.server.luau` | Plugin entry point |
| `src/Core` | Sync engine: tree, snapshots, processor |
| `src/Client` | HTTP + MessagePack transport |
| `src/Lib/MsgPack.luau` | MessagePack encoder/decoder |
| `src/Lib/Dom` | Roblox reflection database and value encoding |
| `src/App` | Fusion based UI (widget, pages, widgets) |

## License

Apache-2.0. Original copyright and license text are preserved in
[LICENSE.md](LICENSE.md); see the repository [NOTICE](../NOTICE).
