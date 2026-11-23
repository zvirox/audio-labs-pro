# `alp-session`

## Purpose

The headless project session and coordination layer. Extracted from `alp-ui` during Phase M1 of the UI migration, `alp-session` encapsulates all non-rendering application state:
- Live `EditSession` managing the active `.alp` project and undo/redo `Command` history.
- Hosting of the authenticated `alp-api::control` Unix domain socket.
- Background worker task dispatch for audio export, peak computation, and model acquisition.
- Project autosave debouncing, crash recovery validation, and workspace configuration persistence (`~/.config/alp/workspace.json`).
- Revision-tagged state snapshotting for live MCP agent attach workflows.

By decoupling session state from presentation, `alp-session` enables pure headless testing and supports both the legacy iced interface during migration and the target Tauri 2 / React desktop shell.

## Public API

`alp-session` is a library crate consumed by `alp-desktop` (and temporarily `alp-ui`). Planned public interfaces include:

- `session::EditSession` — Active project model owner, undoable command execution, clip selection, and clipboard buffers.
- `session::CommandStack` — Linear undo/redo stack maintaining cached replacement clip lists across operations.
- `session::WorkerQueue` — Thread pool for non-blocking offline rendering, format conversions, and peak generation.
- `session::ControlHost` — Authenticated native socket server allowing local tools and `alp-mcp` to interact with the live session.
- `session::WorkspaceState` — Schema-validated persistence for layout, theme, and window preferences.
- `session::AutosaveCoordinator` — Periodic and edit-triggered atomic project snapshotting.

## Data flow

1. Inbound requests arrive via IPC commands (`alp-desktop`) or native socket requests (`alp-api`).
2. `EditSession` verifies preconditions, generates revision IDs, and executes `alp-core::Command` instances.
3. Mutations update in-memory project structures and schedule atomic autosave flushes.
4. Audio playback parameters and mix graph modifications are pushed to `alp-engine` over lock-free `rtrb` rings.
5. Session change events and batched meter data are published back to consumers via callbacks and event queues.

## Allowed dependencies

- `alp-core` — Project structures, commands, clip models, and undo types.
- `alp-dsp` — Short-Time Fourier Transform (STFT) engine and spectrogram computation.
- `alp-engine` — Real-time audio engine, mixer, and transport control.
- `alp-media` — Peak pyramid generation, audio decoding, and file export.
- `alp-voice` — Profile store and TTS/STT synthesis coordination.
- `alp-api` — Control socket definitions and wire protocols.
- **Strictly forbidden:** `iced`, `tauri`, `alp-ui`, `alp-ui-kit`, or any GUI/windowing framework.

## Threading / real-time notes

- `alp-session` runs on the main coordination thread and background worker threads.
- It never touches the real-time audio thread callback directly; all communication with `alp-engine` occurs across non-blocking, lock-free ring buffers or atomic flags.
- Worker tasks are cancelled cooperatively via cancellation tokens.

## Error handling

- All fallible operations return structured `Result<T, SessionError>` types using `thiserror`.
- File corruption, invalid command parameters, and socket disconnections fail gracefully with typed diagnostics.
- Autosave recovery strictly verifies schema integrity before overwriting project state.

## Testing

- Headless unit and integration tests executing full command chains without a display server.
- Concurrency tests verifying lock-free ring buffer communication with mock audio callbacks.
- Autosave simulation verifying zero data loss upon abrupt session termination.

## Known limits

- Single-project session per process instance (multi-document tabs not supported in v1).
- Memory-bound clip fragments; very large projects rely on disk-backed audio block stores.
