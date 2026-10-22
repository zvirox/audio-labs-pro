# 0002 — UI toolkit: iced 0.14 (D2)

- Status: accepted
- Date: 2026-10-22
- Spike: `spikes/ui-iced`

## Context

The plan targets iced but requires a Phase 0 spike against Tauri 2 and Slint,
covering a transparent frosted panel, a scrolling waveform, and blur access.

## Options

| Option | Glass | Waveform | Blur access | Risk |
|---|---|---|---|---|
| iced 0.14 (wgpu) | transparent window + in-app tint verified | custom canvas hit 60 fps | no public raw window handle | upstream API churn between releases |
| Tauri 2 | CSS backdrop-filter is compositor-dependent; WebKitGTK bugs on NVIDIA | canvas fine | via JS/window APIs | WebKitGTK rendering, extra process, larger footprint |
| Slint | native rendering, royalty-free but not Apache-compatible for all uses | custom rendering possible | limited | license model conflicts with Apache-2.0 shipping |

## Decision

iced, pinned to `=0.14.0`, with the `tokio` feature for timers. Recorded in
`spikes/ui-iced/Cargo.toml`. Upgrades are their own task with an ADR.

Measured on Arch Linux / niri 26.04 Wayland / Intel Iris Xe:

- transparent window with a 72%-alpha panel shows desktop content through it;
- 1-hour waveform scroll: 60 fps, 16.6 ms mean frame interval, ~17% of one CPU
  core;
- idle with no animation: 0 frames/s and 0–1% CPU;
- `window::frames()` is self-perpetuating (it keeps 60 fps even when idle) and
  must only be subscribed while something is animating.

## Consequences

- `alp-ui-kit` is built in-house on iced; no raw `iced::widget::button` in
  screens (Phase 2 lint).
- Blur needs a raw window handle. iced 0.14 exposes one through
  `window::run`; see ADR 0012 (which supersedes ADR 0003).
- A small toolkit wrapper in `alp-ui` isolates iced API churn.
- Tauri and Slint are rejected for v1; revisiting them requires a new ADR with
  fresh measurements.
