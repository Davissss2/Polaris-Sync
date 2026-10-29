# Polaris Studio plugin

Roblox Studio plugin for [Polaris](https://github.com/Davissss2/Polaris-Sync),
the Roblox sync tool. It talks to the Polaris CLI and keeps the open place
in sync with the project on disk.

This is a fork of the [Argon](https://github.com/argon-rbx/argon-roblox)
plugin (Apache-2.0), rebranded for Polaris.

## Build

Requirements: [Aftman](https://github.com/LPGhatguy/aftman) (or Rojo and
Wally installed manually).

```bash
cd plugin
aftman install
wally install
rojo build -o Polaris.rbxm
```

Then install it:

```bash
# Option A: let the CLI copy it into the Studio plugins folder
polaris plugin install plugin/Polaris.rbxm

# Option B: copy it manually
#   Windows: %LOCALAPPDATA%\Roblox\Plugins\Polaris.rbxm
```

`polaris plugin uninstall` removes it again.

## Usage

1. `polaris serve` in your project.
2. Open the place in Roblox Studio.
3. Click **Polaris** in the Plugins tab and connect to the session.

## Before you publish your own build

`src/App/Assets.json` still points at the icon and banner asset ids that
came with the fork. Replace them with your own uploaded decals so the
plugin shows your branding.

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
