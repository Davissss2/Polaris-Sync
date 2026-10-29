# Polaris for VS Code

Start, stop and inspect [Polaris](https://github.com/Davissss2/Polaris-Sync)
sessions without leaving the editor.

## What it does

- Status bar entry that shows the ports of the running Polaris sessions and
  turns amber when one of them reports an unhealthy session.
- One menu (`Polaris: Open Menu`, or click the status bar) with every action.
- Commands in the palette: `Start Server`, `Stop Server`, `Build Place`,
  `New Project`, `Install Studio Plugin`, `Session Info`.

The extension is a thin wrapper around the CLI: it never reimplements the sync,
it runs `polaris` and reads `polaris list --json`.

## Nothing happens until you are in a project

The extension activates only when the folder you opened contains a Polaris or
Rojo project (`**/default.project.json`, `**/*.project.json`) or when you run
one of its commands. It never starts a server on its own.

## Two windows, two projects

`polaris list` reports every session on the machine, so the extension filters
them by path: a session belongs to a window when its `default.project.json`
lives inside one of that window's folders.

- The status bar and the menu only talk about this window's sessions.
- `Stop Server` stops **one id at a time**, never `--all`, so stopping in one
  window leaves the other project's server running.
- `Session Info` prints this workspace first and lists the others as
  "other windows (left alone)".

The project folder name is shown in the tooltip, which is how you tell `game`
from `my-other-game` without looking at the port.

## Requirements

The `polaris` CLI installed and on your `PATH`. On Windows the one file
installer (`dist/PolarisSetup.exe` in the repository) does that and also drops
the Studio plugin in place. If the binary lives somewhere else, set
`polaris.executable`.

## Getting started

1. Open the folder that contains your project.
2. `Polaris: Start Server` (or click the status bar). A terminal named Polaris
   opens with `polaris serve <project>`.
3. Open Roblox Studio. The plugin connects and the status bar shows the port.
4. `Polaris: Session Info` prints the project, address, pid and health of every
   session.

## Settings

| Setting | Default | Meaning |
| --- | --- | --- |
| `polaris.executable` | `polaris` | Path to the binary |
| `polaris.projectPath` | empty | Folder with `default.project.json`; detected automatically when empty |
| `polaris.refreshInterval` | `3000` | Status bar polling in ms, `0` disables it |

## Notes

- The extension finds `default.project.json` in the workspace root or one level
  deep, which is where `polaris init <name>` puts it.
- `Stop Server` asks first and stops every session, because the CLI only tracks
  sessions by address and id.
- Building the Studio plugin or the CLI is not part of this extension; those
  live in the repository.
