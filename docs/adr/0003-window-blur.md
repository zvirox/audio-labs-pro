# 0003 — Compositor blur for the glass look

- Status: superseded by ADR 0012 (2026-10-22)
- Date: 2026-10-22
- Decider: project owner (option 1)
- Spike: `spikes/wayland-probe`, `spikes/ui-iced`

> **Superseded.** iced 0.14 now exposes safe raw window/display handles through
> `window::run`, so the vendored-patch decision below is cancelled. KWin/X11 blur
> and the Wayland tint fallback are implemented in `alp-ui::glass`; see ADR 0012.

## Context

The frosted-glass look is a product requirement. Blur must be requested from the
compositor on the app's own Wayland surface (or X11 property). iced 0.14
implements `HasWindowHandle` internally in `iced_winit::Window`, but the
application never receives the window object, and `iced_runtime` exposes only
the `raw_window_handle` types — no getter.

Measured on niri 26.04 (Wayland):

```
globals=44
blur-relevant: ext_background_effect_manager_v1 v1
kde_blur=false ext_background_effect=true
```

So the protocol exists, and a transparent iced window composites correctly, but
the app cannot attach `ext_background_effect_surface_v1` to a surface it cannot
name. The fallback (72% tint + in-app frost gradient) is legible but the desktop
behind it stays sharp.

`iced_winit::Window` already implements `HasWindowHandle`; the gap is purely an
exposure problem, which makes a tiny fork viable.

## Options

1. **Vendor a patched `iced_winit`** (MIT) that exposes the window/display
   handles (e.g. a runtime event with `RawWindowHandle` + `RawDisplayHandle`).
   Adds a fork to maintain against every iced upgrade; enables real blur on
   KDE, niri, Hyprland and other `ext-background-effect-v1` compositors, plus
   the X11 `_KDE_NET_WM_BLUR_BEHIND_REGION` property.
2. **Ship tint-only glass for v1**, treat blur as progressive enhancement, and
   adopt upstream support when iced exposes it. Zero maintenance, visibly less
   "frosted".
3. **Switch toolkit** (winit directly, Tauri, Slint). Reopens ADR 0002 and the
   whole UI plan for a visual feature.

## Decision

**Option 1.** Vendor a minimal `iced_winit` fork in `vendor/iced_winit` and add it
through `[patch.crates-io]` in Phase 1, so Phase 2 can build real glass.

Patch scope (keep it this small):

1. After the winit window exists, publish the `wl_display` + `wl_surface`
   pointers (and the X11 window id) into a process-wide `OnceLock` owned by a
   new tiny crate `alp-glass-ffi` (or a public static in `alp-ui`).
2. No other behaviour changes; the patch must remain a pure addition.
3. `alp-ui::glass` reads that handle on the UI thread, opens a foreign
   `wayland_client::Connection`, binds `ext_background_effect_manager_v1`, and
   sets the blur region to the panel rectangles.
4. On X11, set `_KDE_NET_WM_BLUR_BEHIND_REGION` via x11rb instead.
5. If the handle is missing or the protocol is absent, log once and keep the
   tint fallback.

Rebase the fork on every iced upgrade as its own task; if it cannot be kept
current, fall back to option 2 without changing any other code.

## Consequences

- `THIRD_PARTY.md` lists the vendored iced fork (MIT) and the patch scope.
- `scripts/check-deps.sh` must permit the `[patch.crates-io]` entry.
- Blur is never required for readability: contrast targets are met by the tint
  fallback alone, which Phase 0 verified over arbitrary desktop content.
- Phase 2 gate: screenshot on niri/KDE showing blurred desktop behind a panel,
  plus the fallback screenshot, before glass is considered done.
