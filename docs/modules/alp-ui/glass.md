# `alp-ui` — `glass`

## Purpose

`glass` reports how the transparent window background is produced and requests a
real blur where this build can do so safely. It is the only module that reads
raw window/display handles or talks to the compositor beyond iced.

## Public API

- `GlassMode { CompositorBlur, TintFallback }` with `as_str` and `parse`.
- `GlassReport { session, desktop, handle, mode, blur_available, detail }`, plus
  `pending()`, `badge()` and `Display`.
- `probe(&dyn iced::window::Window) -> GlassReport`.
- `resolve(session, desktop, handle, force_tint) -> GlassReport` — pure mode
  resolution for tests.
- `should_attempt_x11_blur(desktop) -> bool`.

`ALP_FORCE_TINT=1` disables any blur attempt. `ALP_METRICS` is unrelated.

## Data flow

`App::WindowOpened` calls `window::run(id, |window| glass::probe(window))`, which
runs on the UI thread with iced's window object. `probe` identifies the raw
handle and display, reads `XDG_SESSION_TYPE`/`XDG_CURRENT_DESKTOP`, and builds a
report. If the handle is X11 and the desktop is KDE/Plasma, it opens a short
`x11rb` connection, finds `_KDE_NET_WM_BLUR_BEHIND_REGION`, and sets it to the
window rectangle. The report is stored in `App` and shown in the chrome badge
and tooltip.

## Allowed dependencies

`iced` raw handle types and `x11rb` (MIT OR Apache-2.0). No ALP crate. It must
not link cpal, inference crates or a Wayland FFI bridge.

## Threading / real-time notes

Probing and the X11 property write happen once on the UI thread while the window
opens. They do not run per frame and cannot touch the audio callback. The X11
connection is short-lived; no handle is stored after the request.

## Error handling

Every failure path returns a `GlassReport` in `TintFallback` mode with a detail
string: missing handle, unsupported handle, X11 connection/geometry/atom/property
error. A failed blur request never crashes the app and never claims blur. The
tint palette keeps text legible without compositor support.

## Testing

Pure tests cover forced tint, Wayland reporting as a fallback (not fake blur),
KDE-only X11 eligibility, and mode round-tripping. The actual X11 property and
compositor blur require a KDE/X11 session and are not exercised in CI; the
Phase 2 results document the environment matrix honestly.

## Known limits

- Real blur is implemented only for KWin on X11. GNOME, niri and Wayland
  sessions use the tint fallback.
- Wayland `ext_background_effect_v1` needs a `wayland-backend` foreign-display
  bridge that is `unsafe`; the workspace forbids unsafe code, so that path is
  blocked and documented in ADR 0012.
- The blur region is the whole window, not per-panel; panels are single surfaces
  inside it.
- The app cannot force a compositor to blur if its effect is disabled.
