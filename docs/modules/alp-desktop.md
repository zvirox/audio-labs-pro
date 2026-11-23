# `alp-desktop`

## Purpose

The native desktop shell application built on Tauri 2. Replacing the legacy iced desktop binary (`alp-ui`), `alp-desktop` acts as the native platform host that:
- Spawns and manages the single transparent, undecorated OS window and embedded webview.
- Implements the typed Inter-Process Communication (IPC) bridge between the web frontend (`frontend/`) and the headless session engine (`alp-session`).
- Houses the Linux desktop integration and compositor glass probe (`desktop::glass`), configuring KWin/X11 blur via `x11rb` or engaging the translucent tint fallback.
- Manages application lifecycle, native menus, file open dialogs, and desktop shortcuts.

## Public API

`alp-desktop` is the main graphical application binary (`main.rs`). Internal modules include:

- `desktop::app` — Tauri application builder, plugin registrations, and lifecycle hooks.
- `desktop::ipc::commands` — Command invocation handlers dispatching frontend requests to `alp-session`.
- `desktop::ipc::events` — Real-time event publishing for revision changes, task progress, and AI attach logs.
- `desktop::ipc::channels` — Batched `Channel` streaming for real-time input/output audio meters and dictation partials.
- `desktop::glass` — Platform window handle inspection and X11 atom manipulation (`_KDE_NET_WM_BLUR_BEHIND_REGION`) using safe `x11rb`.
- `desktop::dialogs` — Native file chooser bridges for project and audio import/export.

## Data flow

1. The embedded WebKitGTK webview issues typed `invoke` calls over the Tauri IPC channel.
2. `alp-desktop` handlers deserialize parameters and forward calls to `alp-session::EditSession`.
3. Return payloads are serialized back to the frontend; binary waveform peak pyramids are transferred via raw `ipc::Response` binary streams.
4. Background engine events (playback state, render progress, meter updates) are pushed to the webview through Tauri events and channels.

## Allowed dependencies

- `alp-session` — Sole domain dependency for project and audio management.
- `tauri` (v2) — Windowing, webview runtime, and IPC primitives.
- `x11rb` — Safe X11 property configuration for KWin blur.
- `serde`, `serde_json`, `thiserror` — Serialization and error propagation.
- **Strictly forbidden:** Direct dependencies on `alp-core`, `alp-engine`, `alp-media`, `alp-voice`, `alp-infer`, or `alp-ui`.

## Threading / real-time notes

- Runs the Tauri main thread and asynchronous Tokio runtime for command handling.
- Never directly accesses the real-time audio thread or audio ring buffers; all audio interaction is mediated through `alp-session`.
- UI meter events are throttled and batched to approximately 30 Hz to avoid webview thread starvation.

## Error handling

- All IPC commands return strongly typed `Result<T, DesktopError>` responses.
- Errors are mapped to structured JSON error objects providing localized summaries and diagnostic suggestions.
- Compositor blur failures gracefully fall back to dark/light tinted glass modes without terminating the window.

## Testing

- Headless IPC unit tests using Tauri's test harness.
- End-to-end integration tests using `tauri-driver` with mock audio hardware.
- Window property verification verifying safe `x11rb` atom manipulation.

## Known limits

- Requires WebKitGTK (`webkit2gtk-4.1` / `javascriptcoregtk-4.1`) on the host system or Flatpak runtime.
- Wayland background blur defaults to tint fallback in accordance with the `unsafe_code = "deny"` invariant.
- Hand-written code must contain zero `unsafe` (window and KWin blur use safe `raw-window-handle` and `x11rb`). Any compiler exception necessitated by Tauri macro expansion is restricted to the specific generated item and tracked in ADR 0013.
