---
name: polaris
description: Drive Polaris, the Roblox sync tool, from the command line. Use when a project has a default.project.json with a DataModel tree, a polaris.toml, or src/<Service> folders, and the user wants to sync local files with Roblox Studio, inspect running sessions, scaffold missing services, build a place, or run Luau in Studio.
---

# Polaris (Roblox sync tool)

Polaris keeps a Roblox project on disk and a place in Roblox Studio in sync.
Everything is done through the `polaris` CLI: one process serves a project,
the Studio plugin connects to it, and files flow both ways.

Prefer the CLI over any other integration. Every command is scriptable:
non-interactive with `--yes`, machine readable with `--json`, and logs go to
stderr so stdout stays parseable.

## Ground rules

1. Always pass `--yes` and `--json` when running from an agent.
2. Read state before mutating it: `polaris list --json`, `polaris info --json`.
3. The plugin does not connect on its own (`AutoConnect` is off). Starting a
   session only opens it; a `clients: []` in `list --json` means no Studio is
   attached and no file will reach the place. Ask the user to press Connect in
   the Polaris panel, or do it through the Studio interface.
4. Check what a project is bound to before connecting it: `info --json` returns
   `gameId` and `placeIds`. A project without them is accepted by any place and
   asks for confirmation, and `KeepUnknowns` merges it with whatever is open.
5. Never edit `default.project.json` by hand if a command can do it
   (`polaris workspace expand`). If you must, keep the `$className: DataModel`
   root and only touch `tree`.
6. Files under `src/<Service>` are the source of truth for that service.
   Creating a folder creates a `Folder` instance; deleting it deletes the
   instance in Studio on the next sync.
7. Never touch the data directory (`~/.polaris`) or `sessions.toml`. Run
   `polaris config -c workspace ...` for a project; without `-c` the setting
   lands in the global config.
8. After changing files, wait for the server to acknowledge before claiming
   success: `polaris info <path> --json` should report the session address, and
   the Studio plugin shows `Changes synced`.

## Mental model

```text
default.project.json   DataModel tree, maps services to folders
src/<Service>/...      one folder per Roblox service
Packages/              Wally dependencies, when enabled
polaris serve          local server, one per project, own port
```

Naming conventions decide the class:

| File or folder | Becomes |
| --- | --- |
| `Foo.luau` | `ModuleScript` named `Foo` |
| `Foo.server.luau` | `Script` |
| `Foo.client.luau` | `LocalScript` |
| `Foo.model.json` | instance tree, `$className` decides the class |
| `Foo.meta.json` / `Foo.data.json` | properties, attributes, tags for `Foo` |
| `Foo.rbxm` / `Foo.rbxmx` | model imported as-is |
| `Foo.txt`, `Foo.md`, `Foo.csv` | `StringValue`, rich text, `LocalizationTable` |

## Common tasks

### Discover what is running

```bash
polaris list --json
polaris info ./my-game --json
```

`list` returns every session with `project`, `address`, `pid`, `running` and
`healthy`. `healthy` is the real check: it probes the server over HTTP, so
`running: true, healthy: false` means a hung process.
`info` returns `services` (name to folder), `kind` (place or model) and the
`address` of the session serving that project, if any.

### Connect Studio

Connecting is a separate, deliberate step. Do not assume a place is syncing
because a session is running.

```bash
polaris list --json              # clients: [] means no Studio is attached
polaris info ./my-game --json    # gameId and placeIds this project is bound to
```

Compare `placeIds` with the place that is open before connecting. A project
that declares neither `gameId` nor `placeIds` will prompt for confirmation and,
if accepted, merges with what the place already has.

### Scaffold a new project

```bash
polaris init ./my-game --json --yes
```

Creates `default.project.json` plus one folder per Roblox service, with every
service of a fresh place already mapped.

### Add missing services to an existing project

```bash
polaris workspace expand ./my-game --json --yes
```

Idempotent. Only adds services that are missing; never rewrites folders that
are already mapped.

### Create an instance

Create a folder or a script file under the right service:

```bash
mkdir -p src/ReplicatedStorage/Packages/Combat
printf 'return {}\n' > src/ReplicatedStorage/Packages/Combat/init.luau
```

For an instance that is not a script, use a model file:

```json
{
  "className": "Part",
  "properties": { "Size": [4, 1, 2], "Anchored": true }
}
```

Save it as `src/Workspace/Floor.model.json`. Property names and value formats
are Roblox property names, resolved through the reflection database, so
`Vector3` accepts `[4, 1, 2]` and `Color3` accepts `[1, 0.5, 0]`.

### Edit properties without replacing the instance

```json
{ "properties": { "Text": "Hello", "TextSize": 24 } }
```

Save as `src/StarterGui/Main/Title.meta.json` to change only those properties.

### Run Luau in Studio

```bash
polaris exec 'print(#game.Workspace:GetChildren())'
```

Requires a connected session. Use it for things the file format cannot express
(selecting assets, generating content with `Instance.new`, reading runtime
values). Prefer files for anything that should survive a restart. `exec` does
not return the output: it confirms the code was sent, and whatever it prints or
throws stays in the Studio output window. Pass a file path instead of inline
code when the snippet has quotes, since the shell eats them.

### Build a place file

```bash
polaris build ./my-game --output build/game.rbxl
```

Extensions: `.rbxl`, `.rbxlx`, `.rbxm`, `.rbxmx`.

### Stop sessions

```bash
polaris stop --all
```

## When something looks wrong

| Symptom | Check |
| --- | --- |
| `No project files found` | The path must contain `default.project.json`, or use `polaris init`. |
| `Port is already in use` | Another project is serving it; `polaris list --json` shows ports. |
| Changes do not appear in Studio | Is the plugin connected? `list --json` shows `clients`; an empty array means nobody is attached. Does `polaris info` report an address? |
| `401` from the server | The session runs with `--secure`; paste the token from `~/.polaris/token` in the plugin settings. |
| A service is missing | `polaris workspace expand`. |
| `Cannot serve non-place project` | Only `DataModel` projects can be served. |
| Duplicate instances | Two sources map the same path; check `$path` entries. |

## Do not

- Do not invent flags. Run `polaris <command> --help` first.
- Do not assume a fixed port; read it from `list` or `info`.
- Do not parse colored log output; use `--json`.
- Do not claim a sync succeeded without a successful command exit code.
