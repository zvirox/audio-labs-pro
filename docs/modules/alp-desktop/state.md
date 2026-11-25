# `alp-desktop::state`

## Purpose

Thread-safe application state container managing shared instances of `EditSession`, `PersistedWorkspace`, and `GlassReport` for the Linear desktop host.

## Public API

- `AppState` — Container wrapping session, workspace layout, and surface report state in `Arc<Mutex<T>>`.
- `AppState::new_demo()` — Initializes default demo session, panel geometry, and runtime surface metadata.

## Data flow

Managed by Tauri via `tauri::Builder::manage(app_state)` and extracted into command handlers via `tauri::State<'_, AppState>`.

## Allowed dependencies

- `alp-session` (`EditSession`, `PersistedWorkspace`)
- `crate::glass::GlassReport`
- Standard library (`std::sync::{Arc, Mutex}`)

## Threading / real-time notes

Locks are acquired strictly around quick state snapshots and mutations; real-time audio rendering proceeds lock-free inside `alp-engine`.

## Error handling

Lock poison states map to user-safe `IpcError` responses.

## Testing

Verified alongside `commands.rs` command roundtrips and `alp-session` tests.

## Known limits

Browser standalone previews use in-memory fallback mocks when Tauri IPC internals are absent.
