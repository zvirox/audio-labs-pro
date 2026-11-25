# `alp-desktop::glass`

## Purpose

Provides deterministic display-session detection and reports the locked opaque Linear window surface configuration to the frontend.

## Public API

- `GlassMode` — Enum (`CompositorBlur` kept for IPC wire compatibility; runtime selects `TintFallback` for solid opaque rendering).
- `GlassReport` — Inspection report containing desktop environment, display server, handle kind, mode, and surface policy details.
- `probe(window)` — Inspects display server and returns an opaque solid `GlassReport` without applying compositor blur atoms.
- `resolve(session, desktop, handle, force_tint)` — Pure deterministic resolution helper.
- `should_attempt_x11_blur(desktop)` — Pure desktop detection helper retained for test coverage.

## Data flow

During Tauri setup in `main.rs`, `probe()` records display environment metadata into `AppState`. Compositor/X11 blur is permanently disabled to enforce the locked solid opaque Linear UI design standard.

## Allowed dependencies

- `raw-window-handle` (0.6)
- `x11rb`
- `serde`
- Standard library (`std::fmt`, `std::env`)

## Threading / real-time notes

Runs once on the UI setup thread during startup and never touches real-time audio threads.

## Error handling

Non-fatal query errors fall back safely to deterministic default `GlassReport` fields without aborting startup.

## Testing

Deterministic unit tests in `crates/alp-desktop/src/glass.rs` verify `resolve()` and `GlassMode` roundtrip serialization.

## Known limits

Window compositor blur is intentionally disabled (`blur_available: false`, `transparent: false` in `tauri.conf.json`) because the Linear UI standard requires solid opaque surfaces.
