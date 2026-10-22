# 0012 — Phase 2 shell, glass path and the unsafe Wayland limit

- Status: accepted
- Date: 2026-10-22
- Phase: 2 (UI shell and glass system)
- Supersedes: ADR 0003 (compositor blur for the glass look)

## Context

Phase 2 needs a transparent window, a component kit, floating panels, a
peak-pyramid waveform, a command palette and persisted layout. ADR 0003 assumed
that iced 0.14 never exposes a window object, so it accepted a vendored
`iced_winit` fork to publish raw handles.

That assumption is no longer true. iced 0.14 provides:

```rust
window::run(id, |window: &dyn iced::window::Window| {
    let raw = window.window_handle()?.as_raw();
    let display = window.display_handle()?.as_raw();
    // ...
})
```

The handles are available safely on the UI thread. The remaining Wayland gap is
different: attaching `ext_background_effect_manager_v1` to the foreign
`wl_display` requires `wayland_backend::client::Connection::from_foreign_display`
(unsafe), and this workspace sets `unsafe_code = "forbid"`.

## Decision

1. Do **not** vendor `iced_winit`; use the safe `window::run` handles.
2. Implement real blur where a safe protocol exists: KWin on X11 via
   `_KDE_NET_WM_BLUR_BEHIND_REGION` using `x11rb` (MIT/Apache-2.0).
3. Keep the tint fallback as a first-class mode. Panel glass alpha is at least
   0.70 in both themes, so text is legible with a sharp desktop behind it.
4. Report the mode honestly in the chrome. Never claim `compositor_blur` unless
   the property request succeeded.
5. Leave Wayland `ext_background_effect_v1` unimplemented until the workspace
   permits a narrowly audited unsafe bridge or a safe wrapper appears.
6. Persist the last glass mode in `workspace.json`, but re-probe it every launch.
7. Treat the rest of Phase 2 as a single-window shell: eight floating panels,
   one `PanelManager`, versioned layout, conditional frames, and kit-only
   controls (`scripts/check-ui.sh`).

## Consequences

- KDE/X11 gets real KWin blur today; GNOME, niri and Wayland sessions get a clean
  tint fallback. The cross-desktop matrix in Phase 2 results records this
  honestly rather than claiming universal blur.
- The `unsafe_code = "forbid"` policy is preserved. A future Wayland bridge must
  be a separate, documented decision with a safety argument and a test plan.
- `glass.rs` is the only module that reads raw handles or talks to the
  compositor. It never runs per frame and never touches audio.
- The app remains usable if the compositor disables blur or the X11 property is
  rejected; the failure is visible in the glass detail tooltip.
- ADR 0003 is superseded. Its vendor-patch plan and `[patch.crates-io]` entry are
  cancelled; the `x11rb` dependency replaces the FFI crate.
- The glass acceptance gate is narrowed to what this build can prove: KWin/X11
  success when available, tint fallback everywhere else, and no false badge.

## Evidence

- `window::run` compiled and ran under niri/Wayland; the probe reported a
  Wayland handle and the tint fallback.
- `x11rb` property path is unit-tested for desktop eligibility and pure mode
  resolution; a real KWin/X11 session is required to verify the compositor
  effect and is listed as a manual matrix item.
- Release measurement: 1-hour waveform autoscroll at 60–61 frames/s (16.6 ms
  average) and 0.00% idle CPU over four seconds.
