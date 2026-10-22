# `alp-ui` — `workspace`

## Purpose

`workspace` persists the shell layout across restarts: schema version, theme,
Reduce-motion setting, last glass mode and all eight panel snapshots. It uses
`alp-core`'s atomic writer so a crash during a drag cannot corrupt the last good
file.

## Public API

- `LAYOUT_VERSION` (currently 1).
- `PersistedWorkspace { version, theme, reduce_motion, glass, panels }`.
- `PersistedWorkspace::new(theme, reduce_motion, glass, panels)`.
- `path(&AppDirs) -> PathBuf` — `<config>/workspace.json`.
- `load(&Path) -> Option<Self>` — ignores missing, corrupt or newer files.
- `save(&Path) -> Result<(), CoreError>`.

## Data flow

`App::boot` discovers `AppDirs`, builds the path, loads an optional layout and
passes it to `PanelManager::restore`. Layout mutations (`open`, collapse, dock,
drag release, reset, theme, Reduce motion, close) call `save_workspace`, which
serializes pretty JSON and writes through `alp_core::write_atomic`. The glass
mode is recorded from the latest probe. A failed save is surfaced in the UI.

## Allowed dependencies

`alp-core` (`AppDirs`, `CoreError`, `write_atomic`), `serde`, `serde_json`,
`std::fs`, and `crate::panels::PanelState`. No iced view code.

## Threading / real-time notes

Saving happens synchronously on the UI thread after a discrete mutation, not per
drag frame. It creates the config directory if needed and writes a sibling temp
file. With eight panels the file is a few kilobytes; this is not audio-thread
work.

## Error handling

`load` returns `None` for I/O, JSON or version mismatch, which means "use
defaults" rather than blocking startup. `save` propagates `CoreError` for I/O or
serialization; `App` stores the message and shows it, and keeps running with an
unsaved layout.

## Testing

Four tests cover save/load round-trip through nested directories, missing and
corrupt files, rejecting a newer schema, and the XDG config path. Temp files
come from `tempfile`.

## Known limits

- There is no migration yet; any version other than 1 is discarded.
- A corrupt file is silently replaced by defaults on the next save; there is no
  backup of the previous good layout.
- Window position, size and monitor id are not persisted.
- Debouncing is unnecessary at this mutation rate but would be needed if mouse
  movement were ever persisted directly.
