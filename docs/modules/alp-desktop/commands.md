# `alp-desktop::commands`

## Purpose

Provides typed Tauri 2 IPC command handlers bridging requests from the React frontend to the headless session layer (`alp-session`).

## Public API

- `get_project_summary(state)` — Returns the active session's authoritative `ProjectSummary`.
- `get_workspace_state(state)` — Returns the persisted layout and preference state (`PersistedWorkspace`).
- `save_workspace_state(workspace, state)` — Persists layout and preferences to `workspace.json`.
- `reset_workspace_layout(state)` — Resets panel geometry to default arrangement and persists to disk.
- `get_glass_report(state)` — Returns the active compositor glass inspection report (`GlassReport`).
- `trigger_demo_edit(app, action, state)` — Applies a timeline modification and broadcasts `session_revision_changed` event.
- `SessionRevisionPayload` — Struct transmitted across the Tauri event bridge.

## Data flow

1. Frontend initiates a typed `invoke` call over the Tauri IPC channel.
2. The command handler acquires a lock on `AppState`.
3. Commands query or mutate `EditSession` / `PersistedWorkspace`.
4. Return values are serialized to JSON; events are broadcasted via `app.emit()`.

## Allowed dependencies

- `alp-session` (only domain dependency)
- `tauri`
- `serde`, `serde_json`
- `crate::glass`, `crate::state`

## Threading / real-time notes

Command handlers execute asynchronously on Tauri's worker thread pool. Real-time audio operations are strictly isolated inside `alp-engine` via lock-free ringbuffers and are never directly invoked or waited on here.

## Error handling

All commands return `Result<T, String>` with informative error strings on poisoned locks or invalid mutations.

## Testing

Exercised via unit tests in `alp-desktop` and end-to-end IPC invocations from the Vite test harness.

## Known limits

Mutations in M2 operate on the demo editing session. Full undo/redo dispatch across all 7 workspaces expands in M3–M6.
