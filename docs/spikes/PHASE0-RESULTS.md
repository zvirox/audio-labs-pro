# Phase 0 — spike results

Machine: Arch Linux, kernel 7.2.11, niri 26.04 (Wayland), Intel TigerLake
(Iris Xe, no discrete GPU), Rust 1.99.1, PipeWire audio. Wayland and X11
(Xwayland) were exercised on this one machine; KDE, GNOME and NVIDIA are **not**
covered yet.

Reproduce: `cd spikes && cargo run -p <crate> --release` (see each spike header).

## 1. UI and glass (iced 0.14.0)

| Measurement | Result |
|---|---|
| Transparent window over desktop | pass — desktop text legible through 72%-alpha panel |
| 1-hour waveform scroll | 60 fps, 16.6 ms mean frame interval |
| CPU while scrolling | 17% of one core |
| Idle with no animation | 0 frames/s, 0–1% CPU |
| Panel drag + resize inside one window | pass |
| Component set (buttons, tabs, modal, toast, palette) | pass |
| X11 path (Xwayland, `DISPLAY=:0`, no Wayland display) | pass — transparent panel renders, HUD 60 fps while scrolling, idle 0 frames/s / 0% CPU |

Findings that change the plan:

- `window::frames()` subscribed permanently keeps the app at 60 fps even when
  nothing changes, because each frame message triggers a redraw. Subscribe only
  while animating.
- iced 0.14 has no public raw window handle, so compositor blur cannot be
  requested (ADR 0003).
- Canvas has no radial gradient in 0.14; frosted effects must be built from
  layered fills or a custom shader.

Evidence: `docs/spikes/evidence/glass-niri-transparent.png` and
`docs/spikes/evidence/glass-xwayland-transparent.png`.

Reproduce everything in one run: `./scripts/phase0-demo.sh`.

## 2. Blur protocols (wayland-probe)

```
session=wayland desktop=niri globals=44
blur-relevant: ext_background_effect_manager_v1 v1
kde_blur=false ext_background_effect=true
```

The protocol is present; the toolkit gap is the blocker. KDE's
`org_kde_kwin_blur_manager` and X11 `_KDE_NET_WM_BLUR_BEHIND_REGION` still need
testing on those desktops.

## 3. Audio engine (cpal 0.18, rtrb 0.4)

| Measurement | Result |
|---|---|
| Playback callback period | 10.66 ms mean, 11.13 ms max |
| Playback callback gaps (>1.5× mean) | 0 over 10 s |
| Underruns in steady state | 0 (only the deliberate tail and startup) |
| Capture | 467 callbacks / 5 s, 10.67 ms mean, 12.35 ms max, 0 gaps |
| Capture WAV | written, 2.39 s of 48 kHz stereo f32 |

## 4. Decode and peaks (symphonia 0.6, i16 pyramid)

| Measurement | Result |
|---|---|
| 1-hour WAV decode | 998 ms (172.8 M frames, 150k packets) |
| Peak pyramid build + cache write | included above; 3.54 MB cache |
| Cache reload | 0.92 ms |
| Pyramid buckets | 885,938 (3 levels, base 256) |

`symphonia` is MPL-2.0: allowed unmodified, recorded in `THIRD_PARTY.md`.

## 5. Models (ort 2.0.0-rc.15)

| Measurement | Result |
|---|---|
| Kokoro-82M ONNX load | 1.1–1.4 s |
| Inference (2.25 s audio) | 589–822 ms, RTF 0.26–0.36 |
| End-to-end text → WAV | pass, MIT G2P (`piper-plus-g2p`), no espeak-ng |
| Output sanity | peak −8.9 dBFS, RMS −23.6 dBFS, ZCR 0.217, flat factor 0.0 |

Gaps: Kitten nano was not executed (same `ort` path; license verified as
Apache-2.0 on the model card); Moonshine was not executed (ADR 0006). Heavy
Qwen3-TTS was not executed: no GPU here, and the candidate Rust example has no
license. Heavy tier is deferred to Phase 8 (ADR 0008).

## 6. Component and motion libraries

- `iced_aw` 0.14.1 tracks iced 0.14 and is the only external widget crate kept as
  an option; `iced-plus` 0.1.0 and `iced_ui` 0.1.9 are rejected for the design
  system.
- lilt 0.8.2 handles 120/200/320 ms transitions with negligible cost; springs are
  implemented in-house (ADR 0007).

## Acceptance against the plan

| Phase 0 criterion | Result |
|---|---|
| Glass + waveform demo on ≥2 desktops, fallback verified | **partial** — Wayland (niri) and X11 (Xwayland) on one machine; fallback verified; KDE/GNOME and blur demo follow in Phase 2 (ADR 0003) |
| Kokoro (or Kitten) speaks a sentence from Rust | **pass** |
| Moonshine transcribes a clip | **fail / not run** |
| One heavy-model path end-to-end | **not run** (no GPU, license-clean path identified) |
| D1–D8 decided or deferred with reasons | **pass** (ADR 0001–0010; D3 heavy runtime deferred) |
| Component + motion choice with measured numbers | **pass** |
| License pass per model | **pass for Kokoro, Kitten, Moonshine, Qwen3-TTS, VoxCPM2** |

## Limitations

- Single compositor (niri) plus Xwayland, single GPU class; no native X11
  session, KDE or GNOME test, and no NVIDIA machine.
- No audio hardware round-trip: playback/capture go through PipeWire's ALSA
  compatibility layer; real device latency and xrun behaviour under load still
  need a soak test.
- Synthetic 1-hour WAV was used for peaks; real MP3/FLAC/OGG fixtures come in
  Phase 1.
