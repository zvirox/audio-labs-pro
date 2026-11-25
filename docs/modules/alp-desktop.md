# `alp-desktop`

## Purpose

Tauri v2 desktop binary (`audio-labs-pro`) hosting the React 19 Linear UI frontend, window lifecycle (`1280x800` frameless opaque window, `1024x640` minimum), IPC command handlers, and background AI/session event pump.

## Public API

- Tauri application binary `audio-labs-pro` (`crates/alp-desktop/src/main.rs`)
- Exposed Tauri commands (`commands.rs`), shared `AppState` (`state.rs`), and opaque surface reporter (`glass.rs`)

## Data flow

Initializes `AppState` backed by `alp-session`, disables native window decorations in favor of the custom 40 px React `TitleBar`, streams live `session_revision` / transport / AI approval events to the WebView, and dispatches user mutations cleanly to `alp-session`.

## Allowed dependencies

- `alp-session`, `alp-api`, `alp-core`, `alp-engine`, `alp-voice`, `alp-mcp`
- `tauri`, `tokio`, `serde`, `serde_json`, `tracing`
- `raw-window-handle`, `x11rb`

## Threading / real-time notes

All Tauri command handlers execute asynchronously or on managed command pools outside the `alp-engine` real-time audio callback thread.

## Error handling

Structured IPC errors serialize cleanly via `IpcError` rather than panicking across the FFI boundary. Zero `unsafe` code is permitted (`#![forbid(unsafe_code)]`).

## Testing

Unit tests across `glass.rs`, `commands.rs`, and `state.rs` execute via `cargo test -p alp-desktop`.

## Known limits

Native frameless drag, minimize, maximize, and close actions are active inside the Tauri runtime and safely no-op in standalone Vite browser previews.
