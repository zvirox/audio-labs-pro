# `alp-desktop::state`

## Purpose

Thread-safe application state container managing shared instances of `EditSession`, `PersistedWorkspace`, and `GlassReport`.

## Public API

- `AppState` — Thread-safe container wrapping session, workspace, and glass state in `Arc<Mutex<T>>`.
- `AppState::new_demo()` — Initializes default state with a valid demo session and layout.

## Data flow

Managed by Tauri using `tauri::Builder::manage(app_state)`. Injected into command handlers via Tauri's `State<'_, AppState>` extractor.

## Allowed dependencies

- `alp-session` (`EditSession`, `PersistedWorkspace`)
- `crate::glass::GlassReport`
- Standard library (`std::sync::{Arc, Mutex}`)

## Threading / real-time notes

Locks are acquired only during command dispatch and released immediately. The audio engine operates completely decoupled via lock-free ringbuffers.

## Error handling

Lock poison errors are caught and transformed into user-friendly error strings returned over IPC.

## Testing

Covered by command handler execution and desktop shell integration tests.

## Known limits

Workspace persistence flushes to disk upon command invocation; debounced auto-flush will be wired in Phase M3.
