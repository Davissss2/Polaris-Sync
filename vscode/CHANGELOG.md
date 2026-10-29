# Changelog

## 0.3.0

- Knows whether Studio is actually attached, not just whether the server is up:
  the status bar adds a crossed plug and turns amber when no editor is
  listening, and the tooltip names the attached clients
- `Session Info` prints the clients of every session
- Requires Polaris 0.1.3 or newer for the client list; older servers keep working
  and simply do not report it

## 0.2.0

- Activates only in folders that contain a Polaris or Rojo project, or when a
  command is run: no status bar in unrelated workspaces and no server started
  behind your back
- Sessions are scoped to the window: only the ones whose project lives inside
  the open folder show up in the status bar, the menu and Session Info
- `Stop Server` stops the session by id instead of `--all`, so another window
  keeps its server running
- Tooltip and menu name the project folder, not just the port

## 0.1.1

- Read the port from the session address: `polaris list --json` does not send a
  `port` field, so the status bar showed `undefined`

## 0.1.0

- Status bar entry with the ports of the running sessions, amber when a session
  reports itself unhealthy
- Menu with start, stop, build, new project, install plugin and session info
- Commands in the palette and a folder action in the explorer
- Settings for the binary path, the project folder and the refresh interval
