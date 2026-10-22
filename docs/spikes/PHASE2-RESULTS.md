# Phase 2 results — UI shell and glass acceptance

- Date: 2026-10-22
- Build: release (`cargo build --release -p alp-ui`)
- Machine: Arch Linux x86_64, kernel 7.2.11, niri 26.04 on Wayland
- Display: 1920x1080 at scale 1.2 (1600x900 logical), Intel graphics
- Toolchain: rustc 1.99.1, edition 2024

## Verdict

The Phase 2 shell is implemented and the headless checks pass. The 1-hour
waveform scrolls at 60–61 frames/s, idle CPU is zero after startup, Inter is
bundled, panels persist through a versioned workspace file, and worker stubs stay
cancellable without blocking the UI.

The glass criterion is met only where this build has a safe protocol: KWin/X11
gets a real `_KDE_NET_WM_BLUR_BEHIND_REGION` request; niri/Wayland and GNOME use
the legible tint fallback. A full KDE/GNOME cross-desktop matrix was not run on
this machine, so that part is an explicit limitation rather than a claimed pass.

## Acceptance summary

| Criterion | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 104 tests pass (30 `alp-ui`, 18 `alp-ui-kit`, Phase 1 suites, 1 doctest) |
| `scripts/check-deps.sh` | pass (9 crates, directions match architecture) |
| `scripts/check-file-length.sh` | pass (no file over 1,500 lines; two soft warnings) |
| `scripts/check-docs.sh` | pass (55 module documents with all eight sections) |
| `scripts/check-ui.sh` | pass (no raw `iced::widget::button` in `alp-ui`) |
| 1-hour waveform scroll | 60–61 frames/s, 16.6–16.7 ms average frame time (first second 45–55) |
| Idle CPU | 0.00% over 7 s (0 scheduler ticks); RSS about 86 MB |
| Layout persistence | versioned atomic `workspace.json`, round-trip test, restored on restart |
| Worker progress/cancel | 4 job tests pass; UI polls only while jobs run |
| Inter | bundled `InterVariable.ttf`, family `Inter Variable` verified with `fc-scan` |
| Reduce motion | toggle and tests; transitions become instantaneous |
| Glass | KWin/X11 property path; tint fallback on niri/Wayland and GNOME |

## Method and evidence

### Shell and layout

`target/release/alp-ui` was run under niri/Wayland. The final screenshot showed:

- one transparent window with the custom chrome and tint fallback badge;
- four visible panels (Media Bin, Transport, Inspector, Tracks/Timeline) with no
  clipping at 1163x821 logical;
- bundled Inter text at the expected sizes;
- the 120 px canvas waveform with a 0.0–120.0 s view of the 600 s demo;
- three timeline track rows (Voice, Music, SFX);
- every panel's dock/collapse/close controls and resize grip.

The default workspace opens four of the eight panels. Voice Lab, Effects, Models
and Export Queue are available from the chrome glyphs and the command palette.

### 1-hour waveform

```sh
ALP_DEMO_SECONDS=3600 ALP_DEMO_AUTOSCROLL=1 ALP_METRICS=1 \\
  ALP_AUTOCLOSE_SECS=10 target/release/alp-ui
```

Output (one line per second):

```text
alp-ui frames/s=55 frame_ms=16.63 wave=3600s
alp-ui frames/s=61 frame_ms=16.68 wave=3600s
alp-ui frames/s=61 frame_ms=16.57 wave=3600s
alp-ui frames/s=61 frame_ms=16.72 wave=3600s
alp-ui frames/s=61 frame_ms=16.56 wave=3600s
alp-ui frames/s=60 frame_ms=16.70 wave=3600s
alp-ui frames/s=60 frame_ms=16.70 wave=3600s
alp-ui frames/s=60 frame_ms=16.67 wave=3600s
alp-ui frames/s=61 frame_ms=16.71 wave=3600s
```

The first second includes shader/pipeline warm-up and window setup; repeated runs
report 45–55 frames/s for that second. The app
exited cleanly after ten seconds (`exit:0`). The widget selects one peak level per
pixel scale and touches only the visible buckets; the full source is not walked
per frame.

### Idle CPU and memory

The app was started without diagnostics and sampled through `/proc/<pid>/stat`
after a five-second settling period:

```text
ticks=0 over 7 s
cpu_pct=0.00
VmRSS: 87,464 kB
```

An earlier run sampled 17 ticks (4.25%) in a four-second window immediately after
launch, then zero thereafter; this is treated as startup rendering, not steady
state. The conditional frame subscription is what keeps steady idle at zero.

### Persistence

A run wrote `~/.config/alp/workspace.json` (2,014 bytes) with schema version 1,
theme, Reduce motion, glass mode and all eight panel snapshots. A subsequent
launch logged `restored=true`, applied the saved geometry, and rendered the same
four-panel layout. The round-trip unit test covers nested directory creation,
missing/corrupt files, newer-version rejection and path selection.

### Workers

The import, peak verification and export stubs run on `std::thread` and report
through `std::sync::mpsc`. Tests cover progress/completion, cooperative cancel,
failure text and cleanup. The Export panel shows percentage, detail and a Cancel
button; the UI drains events from frames without waiting.

### Fonts

`fc-scan` on the bundled asset reports family `Inter Variable`. The font constants
and all text helpers use that exact name; a wrong name would silently fall back
to a system font. The OFL text is bundled next to the asset and listed in
`THIRD_PARTY.md`.

## Glass matrix

| Session | Handle | Mode | Blur |
|---|---|---|---|
| niri / Wayland (tested) | Wayland | `tint_fallback` | no |
| GNOME / Wayland | Wayland | `tint_fallback` | no supported protocol in this build |
| KDE / X11 | Xlib/Xcb | `compositor_blur` after property success | `_KDE_NET_WM_BLUR_BEHIND_REGION` |
| KDE / Wayland | Wayland | `tint_fallback` | unsafe bridge not allowed |
| barrier X11 | Xlib/Xcb | `tint_fallback` | KWin property not attempted |

`ALP_FORCE_TINT=1` is the reproducible fallback path. The badge only says
`Glass: compositor blur` after `change_property32` and `flush` succeed.

## Limitations and follow-ups

- **Cross-desktop matrix:** KDE, GNOME and X11 sessions were not available here.
  The KWin property code compiles and its eligibility/mode logic is unit-tested,
  but the actual compositor effect needs a manual run on KDE/X11. GNOME has no
  supported blur path in this build.
- **Unsafe Wayland bridge:** attaching `ext_background_effect_v1` to a foreign
  `wl_display` requires `unsafe`; the workspace forbids it. ADR 0012 records the
  decision and a future scoped-bridge option.
- **Stubs:** transport and timeline are visual; they do not connect to
  `alp-engine` or decode a real project. Real editing starts in Phase 3.
- **Accessibility:** keyboard shortcuts, focus for the palette, contrast-safe
  tint and Reduce motion are present, but there is no full accessibility tree,
  screen-reader labelling or in-app text scaling.
- **Motion:** panel open/close uses a lilt offset; drag settle snaps immediately
  and no spring is implemented.
- **File size:** `crates/alp-ui/src/app.rs` is 1,487 lines. Split views before the
  next feature so the 1,500-line guard is not the forcing function.
- **Diagnostics:** frame metrics are emitted to stderr only; there is no in-app
  performance graph.
