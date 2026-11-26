# `alp-desktop` / `plugin_window`

## Purpose

Safe X11 bridge for parenting native plugin editor windows under the studio window. No `unsafe` code is used in this crate.

## Public API

- `PluginWindowRegistry { host_xid, list, register, unregister }`.
- `PluginWindowInfo { plugin_id, xid }`.
- `host_window_id<W: HasWindowHandle>(window) -> Result<u32, String>`.
- `embed_editor(registry, plugin_id, xid, x, y, width, height) -> Result<(), String>`.
- `release_editor(registry, plugin_id) -> Result<(), String>`.

## Data flow

1. At Tauri setup, the main window handle is read via `raw-window-handle`; Xlib/Xcb handles yield a 32-bit XID stored in `AppState.plugin_windows`.
2. A frontend command supplies a plugin id and the plugin editor's XID plus geometry.
3. `x11rb` reparents, configures and maps the editor under the host window.
4. The registry records the editor only after X11 accepts all requests.

## Allowed dependencies

`x11rb`, `raw-window-handle`, `serde` and `tauri`. No new internal crate dependency; `alp-desktop` still depends only on `alp-session`.

## Threading / real-time notes

Commands run on Tauri worker threads. The registry is behind `Arc<Mutex<...>>`; the audio thread never touches it. X11 calls are synchronous IPC to the display server.

## Error handling

Non-X11 handles, missing host id, X11 connection failures, and rejected requests return descriptive strings. The registry is not mutated on failure.

## Testing

Tests verify registry add/list/remove semantics and that `embed_editor` refuses to run before a host XID is captured. Live X11 reparenting requires a display and is not exercised in CI.

## Known limits

- X11/XWayland only; Wayland-native plugin GUI embedding needs a different protocol and is not implemented.
- The host XID must be captured at startup; multi-window scenarios are not supported.
- VST3 descriptors currently cannot be instantiated, so no real VST3 editor uses this bridge yet.
