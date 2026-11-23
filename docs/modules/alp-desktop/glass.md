# `alp-desktop::glass`

## Purpose

Negotiates desktop window blur with the Linux display compositor and provides a guaranteed translucent tint fallback.

## Public API

- `GlassMode` — Enum (`CompositorBlur` or `TintFallback`).
- `GlassReport` — Detailed inspection report describing desktop session, window handle, active mode, and diagnostic reasoning.
- `probe(window)` — Probes the active Tauri window, attempts KWin blur on X11, or selects tint fallback.
- `enable_x11_blur(window)` — Sets `_KDE_NET_WM_BLUR_BEHIND_REGION` on X11 windows using safe `x11rb`.
- `resolve(session, desktop, handle, force_tint)` — Pure resolution logic for deterministic evaluation.
- `should_attempt_x11_blur(desktop)` — Identifies KDE Plasma desktop environments.

## Data flow

During Tauri window setup, `probe()` queries `raw-window-handle 0.6`. If running on KDE/X11 and `ALP_FORCE_TINT` is unset, `enable_x11_blur()` applies the X11 blur atom. The resulting `GlassReport` is stored in `AppState` and reported to the frontend.

## Allowed dependencies

- `raw-window-handle` (0.6)
- `x11rb`
- `serde`
- Standard library (`std::fmt`, `std::env`)

## Threading / real-time notes

Probing runs once during application startup or display migration on the UI thread. It never touches audio callback threads.

## Error handling

If X11 connection fails, window ID conversion fails, or atom configuration is rejected, the probe logs a warning and falls back to `TintFallback` without crashing.

## Testing

Deterministic unit tests in `crates/alp-desktop/src/glass.rs` test `resolve` across Wayland, X11, GNOME, KDE, and `ALP_FORCE_TINT=1`.

## Known limits

Wayland blur protocols (`ext_background_effect_v1`) remain out of scope in accordance with ADR 0012; Wayland sessions use the high-contrast tint fallback ($\alpha \ge 0.70$).
